"""Offline research probes: selected source functions, synthetic data, stubbed effects.

Never imports a legacy application module, reads runtime config, or contacts a platform.
These are observations of legacy behavior, not tests certifying trading correctness.
"""
import ast
from collections import defaultdict
from datetime import datetime, timedelta, timezone
import hashlib
import json
from pathlib import Path
import statistics
import time
import types

WORKSPACE = Path(__file__).resolve().parents[2]
EVIDENCE = {}


def functions(relative, names, env=None):
    path = WORKSPACE / relative
    raw = path.read_bytes()
    tree = ast.parse(raw.decode('utf-8-sig'))
    nodes = [n for n in tree.body if isinstance(n, (ast.FunctionDef, ast.AsyncFunctionDef)) and n.name in names]
    assert {n.name for n in nodes} == set(names)
    # Only vetted stdlib import inside the selected payment function is permitted.
    for node in nodes:
        for inner in ast.walk(node):
            if isinstance(inner, ast.Import):
                assert all(a.name == 'threading' for a in inner.names)
            assert not isinstance(inner, ast.ImportFrom)
    future = ast.ImportFrom(module='__future__', names=[ast.alias(name='annotations')], level=0)
    module = ast.fix_missing_locations(ast.Module(body=[future] + nodes, type_ignores=[]))
    scope = dict(env or {})
    exec(compile(module, str(path), 'exec'), scope)
    EVIDENCE[relative] = {'sha256': hashlib.sha256(raw).hexdigest(), 'symbols': {n.name: n.lineno for n in nodes}}
    return scope


def probe():
    report = []
    env = {'defaultdict': defaultdict, 'datetime': datetime, 'time': time,
           'apply_currency': lambda prices, currency, rate: (prices, currency), 'USD_TO_CNY_DEFAULT': 7.2}
    c = functions('AetherSwap_audit/app/candidate_report.py', [
        '_parse_history_date', 'build_daily_curve', '_percentile', 'seven_day_risk',
        'build_seven_day_stress', '_candidate_decision_notes', 'candidate_snapshot'], env)
    curve = c['build_daily_curve']([['Jan 01 2026', 100, 0], ['Jan 01 2026', 200, 10]], 'CNY')
    assert curve[0]['price'] == 210
    report.append({'id':'L03', 'observation':'混合零成交量记录使日均价 210 高于所有输入价的最大值 200', 'value':curve[0]['price']})
    history = [[f'Jan {i:02d} 2026',100,10] for i in range(1,10)]
    candidate = c['candidate_snapshot']({'goods_id':1,'name':'synthetic','_history':history,'_history_currency':'CNY',
        '_buff_lowest_price':50,'_reference_price':100,'_steam_sell_data':{'spread_rate':0.01,'highest_buy_price':99},'_buff_sell_orders':[]})
    assert candidate['risk']['return_p10'] == 0 and candidate['grade'] == 'B'
    assert candidate['available_qty'] == 1
    report += [{'id':'L04','observation':'P10=0 被 or -1 当成缺失，未进入满足其他 A 条件的等级','value':candidate['grade']},
               {'id':'L05','observation':'空 BUFF 卖单仍展示 available_qty=1','value':candidate['available_qty']}]
    sparse = [{'date':f'2026-01-{i*2+1:02d}','price':100 if i<7 else 80} for i in range(8)]
    risk = c['seven_day_risk'](sparse,100)
    assert risk['return_p10'] == -0.2
    report.append({'id':'L06','observation':'稀疏日曲线的七条间隔实为 14 个自然日，仍进入 seven_day_risk','calendar_days':14,'return_p10':risk['return_p10']})

    env = {'statistics':statistics,'defaultdict':defaultdict,'datetime':datetime,'timedelta':timedelta,'timezone':timezone,
           '_parse_item_date':lambda s:datetime.fromisoformat(s), '_apply_currency':lambda ps,c,r:(ps,c),
           'CV_STABLE_LOW':.05,'CV_STABLE':.15,'MIN_TRADES':5,'MIN_DAILY_TRADES':5,'TRIM_RATIO':.1,
           'SLOPE_DAYS':7,'SLOPE_DOWN_THRESHOLD':-.5,'R2_TREND_THRESHOLD':.6,'USD_TO_CNY':7.2,
           'STATUS_STABLE':'STABLE','STATUS_RISING':'RISING','STATUS_FALLING':'FALLING','STATUS_CHAOS':'CHAOS','STATUS_UNKNOWN':'UNKNOWN'}
    s = functions('AetherSwap_reference/analysis/stability.py',['_percentile','_iqr_bounds','clean_prices_iqr',
        '_vwap_iqr','_safe_volume','_linear_regression_slope','_linear_regression_r_squared','_ema',
        '_analyze_market_status','_daily_avg_prices_last_n','analyze_by_time'],env)
    now = datetime.now(timezone.utc)
    data = [[(now-timedelta(days=i,hours=h)).isoformat(),100,100] for i in range(29) for h in (0,1)]
    analysis = s['analyze_by_time'](data,days=30,currency='CNY',current_price=100)
    assert analysis['count'] == 58 and analysis['total_volume'] == 5800 and not analysis['is_stable']
    report.append({'id':'L07','observation':'min_daily_trades 门槛使用价格记录条数而非成交量','record_count':analysis['count'],'volume':analysis['total_volume'],'stable':analysis['is_stable']})

    rows=[]
    env={'time':time,'_do_payment_notify_and_wait':lambda *args,**kwargs:True,
         'update_checkout':lambda *args,**kwargs:None,'resolve_checkout':lambda *args,**kwargs:None}
    r=functions('AetherSwap_reference/app/pipeline_steps.py',['_do_wait_payment_and_append'],env)
    paid=r['_do_wait_payment_and_append'](object(),{'name':'synthetic'}, {'buff':{'auto_ask_seller_to_send':False}},
        10,1,1,'https://example.invalid','fixture','synthetic-order','synthetic-intent',0,
        lambda *a:None,lambda *a,**k:True,lambda *a:None,lambda:False,rows.append,None,'csgo',market_price=20)
    assert paid==10 and len(rows)==1
    report.append({'id':'L01','observation':'reference 用户确认 True 后记账；提供的 client 无任何查询方法仍能完成','recorded':len(rows)})
    noop=lambda *a,**k:None
    common={'time':time,'get_payment_confirm_reason':lambda:'cancel','_PAYMENT_UNAVAILABLE_REASONS':set()}
    a=functions('AetherSwap_audit/app/pipeline_steps.py',['_do_payment_notify_and_wait'],common)
    checks=[]
    buyer=types.SimpleNamespace(is_order_paid=lambda *a:checks.append('queried') or False,cancel_order=lambda *a:False)
    waits=iter([True,False])
    args=( {'name':'synthetic'},{'notify':{}},10,1,'https://example.invalid','fixture','synthetic-order',0,
           noop,lambda *a,**k:next(waits),noop,lambda:False,None)
    allowed=a['_do_payment_notify_and_wait'](*args,buff_client=buyer)
    assert allowed is False and checks==['queried']
    report.append({'id':'L01-audit','observation':'audit 单笔路径查询 is_order_paid，False 不推进','allowed':allowed})
    email_args=list(args);email_args[1]={'notify':{'email_user':'synthetic','email_pass':'synthetic'}}
    try:
        a['_do_payment_notify_and_wait'](*email_args,buff_client=buyer)
    except UnboundLocalError:
        report.append({'id':'L02','observation':'audit 邮件分支在局部 import threading 前使用 threading，抛出 UnboundLocalError'})
    else:
        raise AssertionError('Expected legacy email branch failure changed')
    return {'scope':'Offline synthetic probes; date/currency dependencies stubbed; no legacy startup or real platform verification',
            'observations':report,'sources':EVIDENCE}


if __name__ == '__main__':
    result=probe()
    target=Path(__file__).resolve().parents[1]/'docs'/'audit'/'probe-results.json'
    target.parent.mkdir(parents=True,exist_ok=True)
    target.write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print(f'PASS: {len(result["observations"])} offline observations -> {target}')
