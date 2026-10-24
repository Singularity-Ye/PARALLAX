// Display metadata only; Rust validates supported units and performs all FX math.
export const currencies = [
  ['CNY', '人民币'], ['UAH', '乌克兰格里夫纳'], ['USD', '美元'],
  ['EUR', '欧元'], ['GBP', '英镑'], ['HKD', '港元'], ['TWD', '新台币'],
];
export function currencyOptions(select, value = 'CNY') {
  select.replaceChildren(...currencies.map(([code, name]) => {
    const option = document.createElement('option');
    option.value = code;
    option.textContent = `${name} ${code}`;
    return option;
  }));
  select.value = value;
}
