import {currencyOptions} from './currencies.js';

function node(tag, text) {
  const element = document.createElement(tag);
  if (text !== undefined) element.textContent = text;
  return element;
}
// Field name, label, minimum, initial value, optional maximum. No business math.
const pricing = [
  ['wall_volume_threshold', '价格墙累计量', 1, 20],
  ['max_ignore_volume', '断层最大阻挡量', 0, 4],
  ['min_lowest_tier_volume', '剔除最低档量 ≤', 0, 3],
  ['min_step_minor', '压价步长（CNY 分）', 1, 1],
  ['min_floor_minor', '价格下限（CNY 分）', 1, 3],
  ['offset_minor', '价格偏移（CNY 分，可负）', -1e12, 0],
];
const fees = [
  ['platform_bps', '平台费用 / bps', 0, 500, 10000],
  ['publisher_bps', '发行商费用 / bps', 0, 1000, 10000],
  ['platform_min_minor', '平台最低费（原币最小单位）', 0, 7],
  ['publisher_min_minor', '发行商最低费（原币最小单位）', 0, 7],
  ['increment_minor', '净额步长（原币最小单位）', 1, 1],
];
function editor(host, specs) {
  const controls = new Map();
  host.replaceChildren();
  for (let i = 0; i < specs.length; i += 2) {
    const row = node('div');
    row.className = 'pair';
    for (const [key, label, min, value, max = 1e12] of specs.slice(i, i + 2)) {
      const field = node('label', label), input = node('input');
      Object.assign(input, {name:key, type:'number', min, max, step:'1', required:true});
      // Set the reset target as well as the current value.
      input.defaultValue = String(value);
      input.value = value;
      controls.set(key, input);
      field.append(input);
      row.append(field);
    }
    host.append(row);
  }
  return {
    read: () => Object.fromEntries([...controls].map(([key, input]) => [key, Number(input.value)])),
    set(value) {
      for (const [key, input] of controls) if (value[key] !== undefined) input.value = value[key];
    },
  };
}
export function pricingEditor(host) {
  const fields = editor(host, pricing);
  return {read: () => ({currency:'CNY', ...fields.read()}), set:fields.set};
}
export function feeEditor(host, {chooseCurrency = false} = {}) {
  const fields = editor(host, fees);
  let currency = 'CNY', select;
  if (chooseCurrency) {
    const label = node('label', '费用模型原币（须与实际盘口/历史一致）');
    select = node('select');
    select.name = 'fee_currency';
    currencyOptions(select);
    label.append(select);
    host.prepend(label);
  }
  function setCurrency(value) {
    currency = value;
    if (select) select.value = value;
  }
  return {
    read: () => ({currency:select ? select.value : currency, ...fields.read()}),
    set(value) { setCurrency(value.currency); fields.set(value); },
    setCurrency,
  };
}
