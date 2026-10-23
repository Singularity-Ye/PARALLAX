"""Read local source trees without executing them or reading account configuration."""
import argparse
import ast
import hashlib
from pathlib import Path
import subprocess


def git(root, *args):
    return subprocess.check_output(['git', '-C', str(root), *args]).decode('utf-8').strip()


def category(path):
    if path.startswith('tests/'):
        return '测试与行为合同'
    if path.startswith('web/'):
        return '前端'
    if path.startswith(('images/', 'flags/', 'output_cards/')):
        return '素材与样例'
    if path.startswith('steamdt/') or path.endswith('iflow_client.py'):
        return '行情来源'
    if path.startswith('analysis/') or path.endswith(('strategy_engine.py', 'purchase_limit.py', 'candidate_report.py', 'money.py', 'trend.py')):
        return '策略与估值'
    if 'auth' in path or path in ('app/accounts.py', 'steam/session.py', 'app/steam_confirm.py'):
        return '账户与认证'
    if path.startswith('buff/') or 'buff_client' in path or 'checkout_guard' in path:
        return 'BUFF 协议与执行保护'
    if path.startswith('steam/') or path in ('app/shared_market.py', 'app/inventory_cs2.py'):
        return 'Steam 行情与库存协议'
    if any(x in path for x in ('gift', 'steam_deals', 'deal_cards')):
        return '游戏折扣与礼物'
    if path.startswith('app/routes/'):
        return 'HTTP 入口'
    if path.startswith('app/services/') or path.startswith('utils/'):
        return '基础设施与后台任务'
    if any(x in path for x in ('database', 'trade_ledger', 'state', 'config')):
        return '持久化与配置'
    if path.startswith('app/') and any(x in path for x in ('pipeline', 'receive_flow', 'sync_sold', 'steam_listings', 'steam_delist', 'repair_error')):
        return '交易生命周期'
    return '启动、包入口与工程资产'


def inspect(path):
    data = path.read_bytes()
    digest = hashlib.sha256(data).hexdigest()
    tree = ast.parse(data.decode('utf-8-sig'), filename=str(path))
    symbols, routes, imports = [], [], set()
    for n in ast.walk(tree):
        if isinstance(n, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
            symbols.append((n.lineno, n.name))
        if isinstance(n, ast.Import):
            imports.update(a.name for a in n.names)
        if isinstance(n, ast.ImportFrom) and n.module:
            imports.add('.' * n.level + n.module)
        if isinstance(n, (ast.FunctionDef, ast.AsyncFunctionDef)):
            for d in n.decorator_list:
                if isinstance(d, ast.Call) and isinstance(d.func, ast.Attribute) and d.func.attr in {'get', 'post', 'put', 'patch', 'delete'} and d.args and isinstance(d.args[0], ast.Constant):
                    routes.append(f'{d.func.attr.upper()} {d.args[0].value} → {n.name}:{n.lineno}')
    return digest, sorted(symbols), routes, sorted(imports), tree


def build(roots):
    out = ['# Parallax 原项目源码盘点', '', '生成方式：`python tools/source_inventory.py --output docs/source-inventory.md`。仅静态解析，不导入或运行旧项目。', '',
           '覆盖 Git 跟踪文件和未忽略的新文件；配置、凭据、数据库与图片只记路径，不读取内容。符号盘点完成不等于行为迁移完成；以下行号相对本地工作树，SHA-256 锁定源码内容。', '']
    snapshots = []
    for root in roots:
        names = sorted(set(git(root, 'ls-files', '-z', '--cached', '--others', '--exclude-standard').split('\0')) - {''})
        names = [n for n in names if (root / n).is_file()]
        snapshots.append((root, names))
        py = [n for n in names if n.endswith('.py')]
        out += [f'## {root.name}', '', f'HEAD：`{git(root, "rev-parse", "HEAD")}`；文件 {len(names)}；Python {len(py)}。', '', '本地差异（不会覆盖）：', '```text', git(root, 'status', '--short') or '(clean)', '```', '', '| 路径 | 分组 |', '| --- | --- |']
        out += [f'| `{n}` | {category(n)} |' for n in names]
        for n in py:
            digest, symbols, routes, imports, tree = inspect(root / n)
            out += ['', f'### {n}', '', f'SHA-256：`{digest}`', '', '符号：' + ('、'.join(f'`{name}:{line}`' for line, name in symbols) or '包入口，无定义'), '', '依赖：' + ('、'.join(f'`{x}`' for x in imports) or '无 import')]
            if routes:
                out += ['', '路由：'] + [f'- `{route}`' for route in routes]
            if n == 'app/strategy_engine.py':
                for node in tree.body:
                    if isinstance(node, ast.AnnAssign) and isinstance(node.target, ast.Name) and node.target.id == 'BUILTIN_MODULES' and isinstance(node.value, ast.Dict):
                        out += ['', '内置策略模块（参数仅列字段，不读取运行配置）：', '', '| ID | 名称 | 参数 |', '| --- | --- | --- |']
                        for k, v in zip(node.value.keys, node.value.values):
                            fields = {key.value: val for key, val in zip(v.keys, v.values) if isinstance(key, ast.Constant)}
                            label = fields.get('name')
                            schema = fields.get('params_schema')
                            keys = [x.value for x in schema.keys if isinstance(x, ast.Constant)] if isinstance(schema, ast.Dict) else []
                            out.append(f'| `{k.value}` | {label.value if isinstance(label, ast.Constant) else ""} | {", ".join(keys)} |')
    a, b = snapshots
    an, bn = set(a[1]), set(b[1])
    out += ['', '## 两版差异索引', '', '相同路径不代表内容相同。非 Python 文件仅列结构差异；以下逐一比较 Python、JS、CSS、HTML 文件。', '', '| 路径 | 相对 reference 的 audit 状态 |', '| --- | --- |']
    for n in sorted(an | bn):
        if Path(n).suffix not in {'.py', '.js', '.css', '.html'}:
            continue
        state = 'audit 独有' if n not in an else 'reference 独有' if n not in bn else '相同' if (a[0]/n).read_bytes() == (b[0]/n).read_bytes() else '不同，需合并行为'
        out.append(f'| `{n}` | {state} |')
    return '\n'.join(out) + '\n'


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=Path('docs/source-inventory.md'))
    args = parser.parse_args()
    workspace = Path(__file__).resolve().parents[2]
    report = build([workspace / 'AetherSwap_reference', workspace / 'AetherSwap_audit'])
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(report, encoding='utf-8')
    print(f'Wrote {args.output}: {len(report.splitlines())} lines; Python AST parsing passed.')
