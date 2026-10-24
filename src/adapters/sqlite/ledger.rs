use super::Store;
use crate::{
    application::ports::LedgerRepository,
    domain::{inventory::InventorySnapshot, ledger::*},
};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
fn get(db: &rusqlite::Connection, account: &str, id: &str) -> anyhow::Result<Option<PurchaseLot>> {
    let raw: Option<String> = db
        .query_row(
            "SELECT snapshot FROM purchase_lots WHERE account_id=?1 AND id=?2",
            params![account, id],
            |r| r.get(0),
        )
        .optional()?;
    raw.map(|s| Ok(serde_json::from_str(&s)?)).transpose()
}
impl LedgerRepository for Store {
    async fn create_lot(
        &self,
        account: String,
        request: LotRequest,
        at: u64,
    ) -> anyhow::Result<LedgerOutcome> {
        self.account_db(move|db|{
            request.validate(at).map_err(anyhow::Error::msg)?;
            let tx=rusqlite::Transaction::new_unchecked(db,TransactionBehavior::Immediate)?;
            let existing:Option<String>=tx.query_row("SELECT id FROM purchase_lots WHERE account_id=?1 AND client_key=?2",params![account,request.client_key],|r|r.get(0)).optional()?;
            if let Some(id)=existing {
                let lot=get(&tx,&account,&id)?.ok_or_else(||anyhow::anyhow!("missing lot projection"))?;
                return Ok(if serde_json::to_value(&lot.request)?==serde_json::to_value(&request)? {LedgerOutcome::Saved(LedgerWrite{lot,replayed:true})}else{LedgerOutcome::Conflict("请求键已用于不同采购记录")});
            }
            let duplicate:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM purchase_lots WHERE account_id=?1 AND purchase_ref=?2)",params![account,request.purchase_ref],|r|r.get(0))?;
            if duplicate {return Ok(LedgerOutcome::Conflict("该账户采购凭据已登记，不重复记成本"));}
            let lot=PurchaseLot::new(account.clone(),request,at).map_err(anyhow::Error::msg)?;
            tx.execute("INSERT INTO purchase_lots(id,account_id,client_key,purchase_ref,snapshot) VALUES(?1,?2,?3,?4,?5)",params![lot.id,account,lot.request.client_key,lot.request.purchase_ref,serde_json::to_string(&lot)?])?;
            tx.commit()?;
            Ok(LedgerOutcome::Saved(LedgerWrite{lot,replayed:false}))
        }).await
    }
    async fn append_lot_event(
        &self,
        account: String,
        id: String,
        request: LotEventRequest,
        inventory: Option<InventorySnapshot>,
        at: u64,
    ) -> anyhow::Result<LedgerOutcome> {
        self.account_db(move|db|{
            let tx=rusqlite::Transaction::new_unchecked(db,TransactionBehavior::Immediate)?;
            let Some(lot)=get(&tx,&account,&id)? else {return Ok(LedgerOutcome::Missing)};
            let old:Option<String>=tx.query_row("SELECT snapshot FROM purchase_lot_events WHERE account_id=?1 AND client_key=?2",params![account,request.client_key],|r|r.get(0)).optional()?;
            if let Some(raw)=old {
                let event:LotEvent=serde_json::from_str(&raw)?;
                return Ok(if event.lot_id==id && serde_json::to_value(event.request)?==serde_json::to_value(&request)? {LedgerOutcome::Saved(LedgerWrite{lot,replayed:true})}else{LedgerOutcome::Conflict("事件请求键已用于其他内容")});
            }
            if request.expected_revision!=lot.revision {return Ok(LedgerOutcome::Conflict("采购批次已更新，请读取最新版本；未覆盖其他修改"));}
            let next=match apply(&lot,&request,inventory.as_ref(),at) {Ok(l)=>l,Err(e)=>return Ok(LedgerOutcome::Invalid(e))};
            if let LotAction::Receive{asset_ids,..}=&request.action {
                for asset in asset_ids {
                    let used:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM purchase_asset_claims WHERE account_id=?1 AND asset_id=?2)",params![account,asset],|r|r.get(0))?;
                    if used {return Ok(LedgerOutcome::Conflict("资产已绑定其他收货记录；不按同名重复分配"));}
                    tx.execute("INSERT INTO purchase_asset_claims(account_id,asset_id,lot_id) VALUES(?1,?2,?3)",params![account,asset,id])?;
                }
            }
            if let Some((kind,reference))=request.action.external_reference() {
                let used:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM purchase_external_claims WHERE account_id=?1 AND kind=?2 AND reference=?3)",params![account,kind,reference],|r|r.get(0))?;
                if used {return Ok(LedgerOutcome::Conflict("挂牌/成交凭据已登记，不重复归因"));}
                tx.execute("INSERT INTO purchase_external_claims(account_id,kind,reference,lot_id) VALUES(?1,?2,?3,?4)",params![account,kind,reference,id])?;
            }
            let event=LotEvent{id:uuid::Uuid::new_v4().to_string(),lot_id:id.clone(),account_id:account.clone(),recorded_at:at,request:request.clone(),result_revision:next.revision};
            tx.execute("INSERT INTO purchase_lot_events(id,account_id,lot_id,client_key,revision,snapshot) VALUES(?1,?2,?3,?4,?5,?6)",params![event.id,account,id,request.client_key,next.revision,serde_json::to_string(&event)?])?;
            tx.execute("UPDATE purchase_lots SET snapshot=?1 WHERE account_id=?2 AND id=?3",params![serde_json::to_string(&next)?,account,id])?;
            tx.commit()?;
            Ok(LedgerOutcome::Saved(LedgerWrite{lot:next,replayed:false}))
        }).await
    }
    async fn lot(&self, account: String, id: String) -> anyhow::Result<Option<PurchaseLot>> {
        self.account_db(move |db| get(db, &account, &id)).await
    }
    async fn lots(&self, account: String) -> anyhow::Result<Vec<PurchaseLot>> {
        self.account_db(move|db|{
            let mut stmt=db.prepare("SELECT snapshot FROM purchase_lots WHERE account_id=?1 ORDER BY rowid DESC LIMIT 10001")?;
            let lots=stmt.query_map([account],|r|r.get::<_,String>(0))?.map(|r|Ok(serde_json::from_str(&r?)?)).collect::<anyhow::Result<Vec<_>>>()?;
            if lots.len()>10000 {anyhow::bail!("账本超过10000批次，需分页后再汇总，未截断成完整持仓");}
            Ok(lots)
        }).await
    }
    async fn lot_events(&self, account: String, id: String) -> anyhow::Result<Vec<LotEvent>> {
        self.account_db(move|db|{let mut stmt=db.prepare("SELECT snapshot FROM purchase_lot_events WHERE account_id=?1 AND lot_id=?2 ORDER BY revision")?;
            stmt.query_map(params![account,id],|r|r.get::<_,String>(0))?.map(|r|Ok(serde_json::from_str(&r?)?)).collect()}).await
    }
}
