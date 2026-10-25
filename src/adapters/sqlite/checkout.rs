use super::{Store, preflight::account_context};
use crate::{
    application::ports::CheckoutRepository,
    domain::{
        checkout::*,
        funding::{self, *},
        preflight::{self, Preflight},
    },
};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
fn intent(
    db: &rusqlite::Connection,
    account: &str,
    id: &str,
) -> anyhow::Result<Option<PurchaseIntent>> {
    let s: Option<String> = db
        .query_row(
            "SELECT snapshot FROM checkout_intents WHERE account_id=?1 AND id=?2",
            params![account, id],
            |r| r.get(0),
        )
        .optional()?;
    s.map(|s| Ok(serde_json::from_str(&s)?)).transpose()
}
fn head(db: &rusqlite::Connection, principal: &str) -> anyhow::Result<Option<FundingSnapshot>> {
    let s:Option<String>=db.query_row("SELECT snapshot FROM cash_observations WHERE principal=?1 ORDER BY observed_at DESC,rowid DESC LIMIT 1",[principal],|r|r.get(0)).optional()?;
    s.map(|s| Ok(serde_json::from_str(&s)?)).transpose()
}
fn saved<T>(item: T, replayed: bool) -> CheckoutOutcome<T> {
    CheckoutOutcome::Saved(CheckoutWrite { item, replayed })
}
impl CheckoutRepository for Store {
    async fn observe_funding(
        &self,
        account: String,
        request: FundingRequest,
        at: u64,
    ) -> anyhow::Result<CheckoutOutcome<FundingSnapshot>> {
        self.account_db(move|db|{
            if let Err(e)=request.validate(at){return Ok(CheckoutOutcome::Invalid(e));}
            let tx=rusqlite::Transaction::new_unchecked(db,TransactionBehavior::Immediate)?;
            let old:Option<String>=tx.query_row("SELECT snapshot FROM cash_observations WHERE account_id=?1 AND client_key=?2",params![account,request.client_key],|r|r.get(0)).optional()?;
            if let Some(s)=old {let old:FundingSnapshot=serde_json::from_str(&s)?;return Ok(if serde_json::to_value(&old.request)?==serde_json::to_value(&request)? {saved(old,true)}else{CheckoutOutcome::Conflict("请求键已用于不同资金观察")});}
            let Some(context)=account_context(&tx,&account)? else{return Ok(CheckoutOutcome::Missing("账户不存在"))};
            let principal=match buff_principal(&context,at){Ok(p)=>p,Err(e)=>return Ok(CheckoutOutcome::Invalid(e))};
            if head(&tx,&principal)?.is_some_and(|h|request.observed_at<=h.request.observed_at){return Ok(CheckoutOutcome::Conflict("资金观察必须晚于当前快照；不倒写或并列最新余额"));}
            let duplicate:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM cash_observations WHERE principal=?1 AND evidence_ref=?2)",params![principal,request.evidence_ref],|r|r.get(0))?;
            if duplicate{return Ok(CheckoutOutcome::Conflict("资金凭据已登记；请引用原快照或使用新观察"));}
            let v=FundingSnapshot{id:uuid::Uuid::new_v4().to_string(),principal:principal.clone(),observed_by_account_id:account,created_at:at,provenance:"manual_unverified".into(),currency:"CNY".into(),platform_verified:false,request};
            tx.execute("INSERT INTO cash_observations(id,principal,account_id,client_key,evidence_ref,observed_at,snapshot) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![v.id,principal,v.observed_by_account_id,v.request.client_key,v.request.evidence_ref,i64::try_from(v.request.observed_at)?,serde_json::to_string(&v)?])?;
            tx.commit()?;Ok(saved(v,false))
        }).await
    }
    async fn funding_view(
        &self,
        account: String,
        at: u64,
    ) -> anyhow::Result<CheckoutOutcome<FundingView>> {
        self.account_db(move|db|{
            let tx=db.unchecked_transaction()?;
            let Some(context)=account_context(&tx,&account)? else{return Ok(CheckoutOutcome::Missing("账户不存在"))};
            let principal=match buff_principal(&context,at){Ok(p)=>p,Err(e)=>return Ok(CheckoutOutcome::Invalid(e))};
            let reserved:i64=tx.query_row("SELECT COALESCE(SUM(reserved_cents),0) FROM checkout_intents WHERE principal=?1 AND state='reserved'",[&principal],|r|r.get(0))?;
            let v=funding::view(principal.clone(),head(&tx,&principal)?,u64::try_from(reserved)?,at);tx.commit()?;Ok(saved(v,false))
        }).await
    }
    async fn prepare_intent(
        &self,
        account: String,
        request: PrepareRequest,
        at: u64,
    ) -> anyhow::Result<CheckoutOutcome<PurchaseIntent>> {
        self.account_db(move|db|{
            if let Err(e)=request.validate(){return Ok(CheckoutOutcome::Invalid(e));}
            let tx=rusqlite::Transaction::new_unchecked(db,TransactionBehavior::Immediate)?;
            // Retry before freshness checks: a lost response must be recoverable after expiry/revocation.
            let old:Option<String>=tx.query_row("SELECT id FROM checkout_intents WHERE account_id=?1 AND client_key=?2",params![account,request.client_key],|r|r.get(0)).optional()?;
            if let Some(id)=old {let v=intent(&tx,&account,&id)?.ok_or_else(||anyhow::anyhow!("missing intent"))?;return Ok(if serde_json::to_value(&v.request)?==serde_json::to_value(&request)? {saved(v,true)}else{CheckoutOutcome::Conflict("请求键已用于不同采购意图")});}
            let Some(context)=account_context(&tx,&account)? else{return Ok(CheckoutOutcome::Missing("账户不存在"))};
            let principal=match buff_principal(&context,at){Ok(p)=>p,Err(e)=>return Ok(CheckoutOutcome::Invalid(e))};
            let active:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM checkout_intents WHERE principal=?1 AND state='reserved')",[&principal],|r|r.get(0))?;
            if active{return Ok(CheckoutOutcome::Conflict("该BUFF主体已有未结束本地意图；恢复原记录或取消，不能重复预留"));}
            let used:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM checkout_intents WHERE account_id=?1 AND preflight_id=?2)",params![account,request.preflight_id],|r|r.get(0))?;
            if used{return Ok(CheckoutOutcome::Conflict("此预检已绑定意图；取消后也需重新预检，不能重复消费"));}
            let Some(funding)=head(&tx,&principal)?.filter(|h|h.id==request.funding_snapshot_id) else{return Ok(CheckoutOutcome::Conflict("需引用当前BUFF主体最新资金观察，不接受旧高余额"))};
            let raw:Option<String>=tx.query_row("SELECT snapshot FROM preflight_inspections WHERE id=?1",[&request.preflight_id],|r|r.get(0)).optional()?;
            let Some(raw)=raw else{return Ok(CheckoutOutcome::Missing("预检记录不存在"))};
            let source:Preflight=serde_json::from_str(&raw)?;
            if source.request.account_id.as_ref()!=Some(&account){return Ok(CheckoutOutcome::Missing("账户预检记录不存在"));}
            let check=match preflight::inspect(source.request,source.source_evaluation,Some(context),at){Ok(v)=>v,Err(e)=>return Ok(CheckoutOutcome::Invalid(e))};
            let v=match crate::domain::checkout::prepare(account.clone(),principal.clone(),request,funding,check,at){Ok(v)=>v,Err(e)=>return Ok(CheckoutOutcome::Invalid(e))};
            tx.execute("INSERT INTO checkout_intents(id,account_id,principal,client_key,preflight_id,state,reserved_cents,snapshot) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![v.id,account,principal,v.request.client_key,v.request.preflight_id,v.state.key(),i64::try_from(v.reserved_cash_cents)?,serde_json::to_string(&v)?])?;
            tx.commit()?;Ok(saved(v,false))
        }).await
    }
    async fn cancel_intent(
        &self,
        account: String,
        id: String,
        request: CancelRequest,
        at: u64,
    ) -> anyhow::Result<CheckoutOutcome<PurchaseIntent>> {
        self.account_db(move|db|{
            let tx=rusqlite::Transaction::new_unchecked(db,TransactionBehavior::Immediate)?;
            let Some(v)=intent(&tx,&account,&id)? else{return Ok(CheckoutOutcome::Missing("账户意图不存在"))};
            let old:Option<String>=tx.query_row("SELECT snapshot FROM checkout_intent_events WHERE account_id=?1 AND client_key=?2",params![account,request.client_key],|r|r.get(0)).optional()?;
            if let Some(s)=old{let e:IntentEvent=serde_json::from_str(&s)?;return Ok(if e.intent_id==id && serde_json::to_value(e.request)?==serde_json::to_value(&request)? {saved(v,true)}else{CheckoutOutcome::Conflict("事件请求键已用于其他取消操作")});}
            if request.expected_revision!=v.revision{return Ok(CheckoutOutcome::Conflict("意图版本已更新，请恢复当前记录"));}
            let next=match crate::domain::checkout::cancel(&v,&request,at){Ok(v)=>v,Err(e)=>return Ok(CheckoutOutcome::Invalid(e))};
            let event=IntentEvent{id:uuid::Uuid::new_v4().to_string(),intent_id:id.clone(),account_id:account.clone(),recorded_at:at,request,result_revision:next.revision};
            tx.execute("INSERT INTO checkout_intent_events(id,account_id,intent_id,client_key,snapshot) VALUES(?1,?2,?3,?4,?5)",params![event.id,account,id,event.request.client_key,serde_json::to_string(&event)?])?;
            tx.execute("UPDATE checkout_intents SET state=?1,snapshot=?2 WHERE id=?3 AND account_id=?4",params![next.state.key(),serde_json::to_string(&next)?,id,account])?;
            tx.commit()?;Ok(saved(next,false))
        }).await
    }
    async fn purchase_intent(
        &self,
        account: String,
        id: String,
    ) -> anyhow::Result<Option<PurchaseIntent>> {
        self.account_db(move |db| intent(db, &account, &id)).await
    }
    async fn purchase_intents(&self, account: String) -> anyhow::Result<Vec<IntentSummary>> {
        self.account_db(move|db|{let mut stmt=db.prepare("SELECT snapshot FROM checkout_intents WHERE account_id=?1 ORDER BY rowid DESC LIMIT 100")?;stmt.query_map([account],|r|r.get::<_,String>(0))?.map(|r|Ok(IntentSummary::from(serde_json::from_str::<PurchaseIntent>(&r?)?))).collect()}).await
    }
    async fn intent_events(&self, account: String, id: String) -> anyhow::Result<Vec<IntentEvent>> {
        self.account_db(move|db|{let mut stmt=db.prepare("SELECT snapshot FROM checkout_intent_events WHERE account_id=?1 AND intent_id=?2 ORDER BY rowid")?;stmt.query_map(params![account,id],|r|r.get::<_,String>(0))?.map(|r|Ok(serde_json::from_str(&r?)?)).collect()}).await
    }
}
