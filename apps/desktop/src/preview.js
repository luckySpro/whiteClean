const KB = 1024;
const MB = KB * 1024;
const GB = MB * 1024;
const DAY = 24 * 60 * 60 * 1000;
const HOME = "/Users/you";

function file(relative, size, modifiedMs, extra = {}) {
  const path = `${HOME}/${relative}`;
  return {
    path,
    name: path.split("/").pop(),
    size: Math.round(size),
    modified_ms: modifiedMs,
    ...extra,
  };
}

function totals(groups) {
  const items = groups.flatMap((group) => group.items);
  return {
    freeable_bytes: items.reduce((sum, item) => sum + item.size, 0),
    delete_count: items.length,
  };
}

export function previewReport() {
  const now = Date.now();
  const groups = [
    {
      kind: "exact_duplicate",
      keep_name: "Setup (1).dmg",
      keep_path: `${HOME}/Downloads/Setup (1).dmg`,
      note: null,
      items: [file("Downloads/Setup.dmg", 1.4 * GB, now - 40 * DAY)],
    },
    {
      kind: "exact_duplicate",
      keep_name: "旅行视频.mov",
      keep_path: `${HOME}/Documents/旅行视频.mov`,
      note: null,
      items: [
        file("Downloads/旅行视频.mov", 820 * MB, now - 60 * DAY),
        file("Desktop/旅行视频 副本.mov", 820 * MB, now - 58 * DAY),
      ],
    },
    {
      kind: "older_version",
      keep_name: "报告.pdf",
      keep_path: `${HOME}/Documents/报告.pdf`,
      note: "content_differs",
      items: [file("Documents/报告 副本.pdf", 2.1 * MB, now - 12 * DAY)],
    },
    {
      kind: "older_version",
      keep_name: "简历-v3.docx",
      keep_path: `${HOME}/Documents/简历-v3.docx`,
      note: "content_differs",
      items: [
        file("Documents/简历-v2.docx", 480 * KB, now - 90 * DAY),
        file("Documents/简历.docx", 455 * KB, now - 200 * DAY),
      ],
    },
    {
      kind: "old_installer",
      keep_name: null,
      keep_path: null,
      note: null,
      items: [
        file("Downloads/Microsoft_Office_Installer.pkg", 1.1 * GB, now - 160 * DAY),
        file("Downloads/WeChatMac.dmg", 310 * MB, now - 95 * DAY),
      ],
    },
    {
      kind: "regenerable",
      keep_name: null,
      keep_path: null,
      note: null,
      items: [
        file("Library/Caches/Google/Chrome", 1.2 * GB, now, { label: "Chrome 浏览器缓存" }),
        file("Library/Caches/com.tencent.xinWeChat", 960 * MB, now, { label: "微信缓存" }),
        file("Library/Caches/com.apple.Safari", 420 * MB, now, { label: "Safari 缓存" }),
        file("Library/Caches/com.spotify.client", 260 * MB, now, { label: "Spotify 缓存" }),
        file("Library/Logs/DiagnosticReports", 180 * MB, now, { label: "应用日志" }),
        file("Library/Caches/com.apple.QuickLook.thumbnailcache", 96 * MB, now, { label: "预览缩略图" }),
      ],
    },
  ];
  return {
    ...totals(groups),
    skipped_sensitive_count: 2,
    disk: { total_bytes: 494 * GB, free_bytes: 38.2 * GB },
    groups,
    skipped: [
      { name: "劳动合同.pdf", note: "sensitive", path: `${HOME}/Documents/劳动合同.pdf` },
      { name: "身份证扫描.jpg", note: "sensitive", path: `${HOME}/Downloads/身份证扫描.jpg` },
    ],
  };
}

export function emptyReport() {
  return {
    freeable_bytes: 0,
    delete_count: 0,
    skipped_sensitive_count: 0,
    disk: { total_bytes: 494 * GB, free_bytes: 44.4 * GB },
    groups: [],
    skipped: [],
  };
}
