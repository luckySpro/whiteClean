const UNITS = ["KB", "MB", "GB", "TB"];
const DAY = 24 * 60 * 60 * 1000;

export function formatBytes(bytes) {
  const size = Number(bytes) || 0;
  if (size < 1024) {
    return { text: `${size} B`, num: String(size), unit: "B" };
  }
  let value = size / 1024;
  let index = 0;
  while (value >= 1024 && index < UNITS.length - 1) {
    value /= 1024;
    index += 1;
  }
  const num = value.toFixed(value >= 100 ? 0 : 1);
  return { text: `${num} ${UNITS[index]}`, num, unit: UNITS[index] };
}

const PLACES = [
  ["/Downloads/", "下载"],
  ["/Desktop/", "桌面"],
  ["/Documents/", "文稿"],
  ["/Library/Caches/", "缓存"],
  ["/Library/Logs/", "日志"],
];

export function placeOf(path) {
  if (!path) return "";
  const match = PLACES.find(([marker]) => path.includes(marker));
  return match ? match[1] : "";
}

export function agoText(ms, now = Date.now()) {
  const days = Math.floor((now - ms) / DAY);
  if (days < 1) return "今天";
  if (days < 30) return `${days} 天前`;
  if (days < 365) return `${Math.floor(days / 30)} 个月前`;
  return `${Math.floor(days / 365)} 年前`;
}
