export const copy = {
  appName: "WhiteClean",
  safeNote: "只放进废纸篓，随时能撤销",
  scanning: "正在检查这台 Mac",
  scanSteps: ["查找重复的文件", "查找同一文件的旧版本", "查找用不上的安装包", "查找可以重建的缓存"],
  cleaning: "正在放进废纸篓",
  undoing: "正在放回原来的位置",

  freeableLabel: "可以帮你腾出",
  heroSub: (count) => `已经帮你挑好了 ${count} 项，都可以放心删。想留下的，取消勾选就行。`,
  diskLabel: "磁盘剩余",
  diskAfter: (size) => `清理后 ${size}`,

  pickedOf: (picked, total) => `已选 ${picked}/${total}`,
  selectAll: (title) => `全选${title}`,
  showMore: (count) => `再显示 ${count} 项`,
  showLess: "收起",

  dupDetail: (place, keepPlace, keepName) =>
    `${place} · 和${keepPlace === place ? "同一文件夹" : keepPlace}里的「${keepName}」一模一样，留那份`,
  versionDetail: (keepName, differs) =>
    differs ? `旧版本 · 留下较新的「${keepName}」，两份内容不一样` : `旧版本 · 留下较新的「${keepName}」`,
  installerDetail: (place, ago) => `${place} · ${ago}下载的，软件装好后就用不上了`,
  keptTitle: "已帮你留下",
  keptHint: "看起来像证件、合同或密码，不会动它们",
  keptReason: (place) => (place ? `${place} · 像证件或合同，不会动` : "像证件或合同，不会动"),

  footerPicked: (count, size) => `已选 ${count} 项，共 ${size}`,
  reveal: "在访达中显示",
  footerNote: "清空废纸篓之前，都能一键撤销",
  clean: (size) => `清理 ${size}`,
  nothingPicked: "先勾选要清理的项",

  confirmTitle: (count, size) => `把 ${count} 项放进废纸篓，腾出 ${size}？`,
  confirmUndo: "清空废纸篓之前，随时可以一键撤销。",
  confirmSkip: (count) => (count > 0 ? `${count} 个可能重要的文件已经留下，不会动。` : "没有发现敏感文件。"),
  confirmButton: "放进废纸篓",
  cancel: "再看看",

  doneTitle: (size) => `好了，腾出了 ${size}`,
  doneBody: (count) => `${count} 项已经在废纸篓里。清空废纸篓之前，随时可以撤销。`,
  failedSome: (count) => `有 ${count} 项没能移动，已经原样留下。`,
  undo: "撤销",
  finish: "完成",
  undoneTitle: "已经放回原处",
  undoneBody: "所有文件都回到了原来的位置。",
  trashEmptyTitle: "废纸篓已清空",
  trashEmptyBody: "这些文件已经彻底删除，没法撤销了。",

  nothingTitle: "这台 Mac 很干净",
  nothingBody: "现在没有需要清理的，过几天再来看看。",
  rescan: "重新检查",

  permission: "没有权限就看不到桌面和文稿。去系统设置里打开「完全磁盘访问权限」，勾上 WhiteClean 再回来。",
  openSettings: "打开系统设置",

  kinds: {
    exact_duplicate: { title: "重复文件", hint: "同一个文件存了好几份，保留最新的一份", icon: "copy" },
    older_version: { title: "同一文件的旧版本", hint: "带「副本」「(1)」「v2」的旧文件", icon: "layers" },
    old_installer: { title: "用不上的安装包", hint: "放了一个月以上的 .dmg、.pkg", icon: "box" },
    regenerable: { title: "缓存和日志", hint: "软件临时存的东西，删了会自动重建", icon: "wind" },
  },
};
