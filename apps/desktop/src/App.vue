<script setup>
import { computed, nextTick, onBeforeUnmount, onMounted, reactive, ref } from "vue";
import { copy } from "./copy";
import { formatBytes, placeOf } from "./format";
import { emptyReport, previewReport } from "./preview";
import AppIcon from "./components/AppIcon.vue";
import CheckBox from "./components/CheckBox.vue";
import CleanSection from "./components/CleanSection.vue";

const KIND_ORDER = ["exact_duplicate", "older_version", "old_installer", "regenerable"];

const screen = ref("ready");
const report = ref(emptyReport());
const selected = reactive(new Set());
const confirming = ref(false);
const permissionMissing = ref(false);
const trashGone = ref(false);
const cleaned = ref({ bytes: 0, count: 0, paths: [], failed: 0 });
const stepIndex = ref(0);
const confirmButton = ref(null);
const folders = ref([]);
let stepTimer = 0;

const activeFoldersCount = computed(() => folders.value.filter((f) => f.enabled).length);

const sections = computed(() =>
  KIND_ORDER.map((kind) => {
    const rows = report.value.groups
      .filter((group) => group.kind === kind)
      .flatMap((group) =>
        group.items.map((item) => ({
          ...item,
          kind,
          keep_name: group.keep_name,
          keep_path: group.keep_path,
          note: group.note,
        })),
      );
    return { kind, rows };
  }).filter((section) => section.rows.length > 0),
);

const allRows = computed(() => sections.value.flatMap((section) => section.rows));
const pickedRows = computed(() => allRows.value.filter((row) => selected.has(row.path)));
const pickedBytes = computed(() => pickedRows.value.reduce((sum, row) => sum + row.size, 0));
const totalAmount = computed(() => formatBytes(report.value.freeable_bytes));
const pickedText = computed(() => formatBytes(pickedBytes.value).text);
const hasSuggestions = computed(() => allRows.value.length > 0);

const disk = computed(() => {
  const { total_bytes: total, free_bytes: free } = report.value.disk;
  const used = Math.max(total - free, 0);
  const freeing = Math.min(pickedBytes.value, used);
  return {
    usedPct: total ? ((used - freeing) / total) * 100 : 0,
    freeingPct: total ? (freeing / total) * 100 : 0,
    free: formatBytes(free).text,
    after: formatBytes(free + freeing).text,
  };
});

onMounted(async () => {
  const params = new URLSearchParams(window.location.search);
  permissionMissing.value = params.get("permission") === "1";
  trashGone.value = params.get("trash") === "empty";
  window.addEventListener("keydown", onKey);

  await initFolders();

  if (params.get("auto") === "1") {
    await scan({ previewEmpty: params.get("empty") === "1" });
  }
});

onBeforeUnmount(() => {
  window.removeEventListener("keydown", onKey);
  window.clearInterval(stepTimer);
});

function inTauri() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

async function tauriInvoke(command, args) {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke(command, args);
}

function defaultFallbackFolders() {
  return [
    { id: "downloads", name: "下载", path: "/Users/you/Downloads", kind: "default", enabled: true },
    { id: "desktop", name: "桌面", path: "/Users/you/Desktop", kind: "default", enabled: true },
    { id: "documents", name: "文稿", path: "/Users/you/Documents", kind: "default", enabled: true },
    { id: "caches", name: "系统缓存与日志", path: "/Users/you/Library/Caches", kind: "cache", enabled: true },
  ];
}

async function initFolders() {
  let list = [];
  if (inTauri()) {
    try {
      const cfg = await tauriInvoke("get_default_folders");
      list = cfg.folders || [];
    } catch {
      list = defaultFallbackFolders();
    }
  } else {
    list = defaultFallbackFolders();
  }

  try {
    const saved = JSON.parse(localStorage.getItem("whiteclean_custom_folders") || "[]");
    if (Array.isArray(saved)) {
      for (const custom of saved) {
        if (!list.some((item) => item.path === custom.path)) {
          list.push(custom);
        }
      }
    }
  } catch {}
  folders.value = list;
}

async function addCustomFolder() {
  let chosen = null;
  if (inTauri()) {
    try {
      chosen = await tauriInvoke("pick_folder");
    } catch (e) {
      console.error(e);
    }
  } else {
    chosen = window.prompt("请输入要检查的文件夹绝对路径：", "/Users/you/Projects");
  }

  if (!chosen) return;
  const path = chosen.trim();
  if (!path) return;

  if (folders.value.some((f) => f.path === path)) {
    return;
  }
  const name = path.split("/").filter(Boolean).pop() || path;
  folders.value.push({
    id: "custom_" + Date.now(),
    name,
    path,
    kind: "custom",
    enabled: true,
  });
  saveCustomFolders();
}

function removeCustomFolder(id) {
  folders.value = folders.value.filter((f) => f.id !== id);
  saveCustomFolders();
}

function saveCustomFolders() {
  const customs = folders.value.filter((f) => f.kind === "custom");
  try {
    localStorage.setItem("whiteclean_custom_folders", JSON.stringify(customs));
  } catch {}
}

async function startScan() {
  const activeFolders = folders.value.filter((f) => f.enabled && f.kind !== "cache").map((f) => f.path);
  const includeCaches = folders.value.some((f) => f.enabled && f.kind === "cache");
  if (activeFolders.length === 0 && !includeCaches) {
    alert(copy.emptyFoldersWarning);
    return;
  }
  await scan({ folders: activeFolders, includeCaches });
}

async function scan(options = {}) {
  screen.value = "scanning";
  stepIndex.value = 0;
  window.clearInterval(stepTimer);
  stepTimer = window.setInterval(() => {
    stepIndex.value = (stepIndex.value + 1) % copy.scanSteps.length;
  }, 450);
  let next;
  try {
    if (inTauri()) {
      const scanArgs = {};
      if (options.folders) {
        scanArgs.options = {
          folders: options.folders,
          include_caches: options.includeCaches !== false,
        };
      }
      const [result] = await Promise.all([tauriInvoke("scan", scanArgs), wait(700)]);
      next = result;
      permissionMissing.value = Boolean(result.permission_missing);
    } else {
      await wait(1600);
      next = options.previewEmpty ? emptyReport() : previewReport();
    }
  } catch {
    next = emptyReport();
  }
  window.clearInterval(stepTimer);
  report.value = next;
  selected.clear();
  allRows.value.forEach((row) => selected.add(row.path));
  screen.value = "home";
}

async function openSettings() {
  if (inTauri()) await tauriInvoke("open_permission_settings");
}

async function reveal(path) {
  if (!inTauri()) return;
  try {
    await tauriInvoke("reveal_in_finder", { path });
  } catch {
    // The file may already be gone. The list itself stays as it is.
  }
}

function toggleRow(path) {
  if (selected.has(path)) selected.delete(path);
  else selected.add(path);
}

function toggleSection(section) {
  const allOn = section.rows.every((row) => selected.has(row.path));
  section.rows.forEach((row) => (allOn ? selected.delete(row.path) : selected.add(row.path)));
}

async function openConfirm() {
  if (!pickedRows.value.length) return;
  confirming.value = true;
  await nextTick();
  confirmButton.value?.focus();
}

async function confirmClean() {
  confirming.value = false;
  const paths = pickedRows.value.map((row) => row.path);
  screen.value = "working";
  if (inTauri()) {
    try {
      const outcome = await tauriInvoke("clean", { paths });
      cleaned.value = {
        bytes: outcome.bytes,
        count: outcome.trashed,
        paths: outcome.paths,
        failed: outcome.failed,
      };
      screen.value = outcome.trashed > 0 ? "result" : "home";
    } catch {
      screen.value = "home";
    }
    return;
  }
  cleaned.value = {
    bytes: pickedBytes.value,
    count: paths.length,
    paths,
    failed: 0,
  };
  await wait(900);
  screen.value = "result";
}

async function undo() {
  screen.value = "undoing";
  if (inTauri()) {
    try {
      const outcome = await tauriInvoke("undo");
      screen.value = outcome.restored > 0 ? "undone" : "blocked";
    } catch {
      screen.value = "blocked";
    }
    return;
  }
  await wait(800);
  screen.value = trashGone.value ? "blocked" : "undone";
}

function finish() {
  if (screen.value === "result" || screen.value === "blocked") {
    const gone = new Set(cleaned.value.paths);
    const groups = report.value.groups
      .map((group) => ({ ...group, items: group.items.filter((item) => !gone.has(item.path)) }))
      .filter((group) => group.items.length > 0);
    const items = groups.flatMap((group) => group.items);
    report.value = {
      ...report.value,
      groups,
      freeable_bytes: items.reduce((sum, item) => sum + item.size, 0),
      delete_count: items.length,
      disk: { ...report.value.disk, free_bytes: report.value.disk.free_bytes + cleaned.value.bytes },
    };
    gone.forEach((path) => selected.delete(path));
  }
  screen.value = "home";
}

function onKey(event) {
  if (event.key === "Escape" && confirming.value) {
    confirming.value = false;
  }
}

function wait(ms) {
  return new Promise((resolve) => window.setTimeout(resolve, ms));
}
</script>

<template>
  <div class="window">
    <header class="topbar">
      <div class="brand">
        <span class="mark" aria-hidden="true">白</span>
        <span class="brand-name">{{ copy.appName }}</span>
      </div>
      <div class="topbar-right">
        <button v-if="screen === 'home'" class="btn-text" type="button" @click="screen = 'ready'">
          <AppIcon name="folder" :size="15" />{{ copy.reconfigureFolders }}
        </button>
        <span class="topbar-note"><AppIcon name="shield" :size="16" />{{ copy.safeNote }}</span>
      </div>
    </header>

    <!-- Ready / Configuration Screen -->
    <main v-if="screen === 'ready'" class="content">
      <div v-if="permissionMissing" class="notice">
        <AppIcon name="lock" :size="20" />
        <p>{{ copy.permission }}</p>
        <button class="btn btn-small" type="button" @click="openSettings">
          <AppIcon name="settings" :size="16" />{{ copy.openSettings }}
        </button>
      </div>

      <section class="ready-hero">
        <div class="ready-head">
          <h1 class="ready-title">{{ copy.foldersTitle }}</h1>
          <p class="ready-sub">{{ copy.foldersSub }}</p>
        </div>

        <div class="folder-list">
          <div
            v-for="folder in folders"
            :key="folder.id"
            class="folder-row"
            :class="{ disabled: !folder.enabled }"
          >
            <button
              class="folder-check"
              type="button"
              role="checkbox"
              :aria-checked="folder.enabled"
              @click="folder.enabled = !folder.enabled"
            >
              <CheckBox :state="folder.enabled ? 'on' : 'off'" />
            </button>

            <span class="folder-tile" :class="folder.kind === 'cache' ? 'tile-folder' : 'tile-folder'">
              <AppIcon :name="folder.kind === 'cache' ? 'wind' : 'folder'" :size="18" />
            </span>

            <div class="folder-info" @click="folder.enabled = !folder.enabled">
              <div class="folder-name-row">
                <span class="folder-name">{{ folder.name }}</span>
                <span v-if="folder.kind === 'default'" class="folder-tag">常用</span>
                <span v-else-if="folder.kind === 'cache'" class="folder-tag">系统</span>
                <span v-else class="folder-tag custom">自定义</span>
              </div>
              <span class="folder-path">{{ folder.path }}</span>
            </div>

            <button
              v-if="folder.kind === 'custom'"
              class="folder-remove"
              type="button"
              :title="copy.removeFolder"
              @click="removeCustomFolder(folder.id)"
            >
              <AppIcon name="x" :size="16" />
            </button>
          </div>

          <button class="add-folder-btn" type="button" @click="addCustomFolder">
            <AppIcon name="plus" :size="16" />
            <span>{{ copy.addFolder }}</span>
          </button>
        </div>
      </section>

      <footer class="actionbar">
        <div class="actionbar-text">
          <p class="actionbar-main">已选择 {{ activeFoldersCount }} 个检查位置</p>
          <p class="actionbar-note">未勾选的文件夹本次将完全跳过</p>
        </div>
        <button
          class="btn btn-primary btn-large"
          type="button"
          :disabled="activeFoldersCount === 0"
          @click="startScan"
        >
          <AppIcon name="play" :size="18" />{{ copy.startScan }}
        </button>
      </footer>
    </main>

    <!-- Scanning / Cleaning / Undoing State -->
    <main v-else-if="screen === 'scanning' || screen === 'working' || screen === 'undoing'" class="center" aria-live="polite">
      <span class="spinner" aria-hidden="true"></span>
      <h1 class="center-title">
        {{ screen === "scanning" ? copy.scanning : screen === "working" ? copy.cleaning : copy.undoing }}
      </h1>
      <p v-if="screen === 'scanning'" class="center-body">{{ copy.scanSteps[stepIndex] }}</p>
    </main>

    <!-- Result State -->
    <main v-else-if="screen === 'result'" class="center">
      <span class="badge badge-ok"><AppIcon name="check" :size="34" /></span>
      <h1 class="center-title big">{{ copy.doneTitle(formatBytes(cleaned.bytes).text) }}</h1>
      <p class="center-body">{{ copy.doneBody(cleaned.count) }}</p>
      <p v-if="cleaned.failed" class="center-body">{{ copy.failedSome(cleaned.failed) }}</p>
      <div class="center-actions">
        <button class="btn btn-secondary" type="button" @click="undo">
          <AppIcon name="undo" :size="18" />{{ copy.undo }}
        </button>
        <button class="btn btn-primary" type="button" @click="finish">{{ copy.finish }}</button>
      </div>
    </main>

    <!-- Undone / Blocked State -->
    <main v-else-if="screen === 'undone' || screen === 'blocked'" class="center">
      <span class="badge" :class="screen === 'undone' ? 'badge-ok' : 'badge-muted'">
        <AppIcon :name="screen === 'undone' ? 'undo' : 'trash'" :size="30" />
      </span>
      <h1 class="center-title big">{{ screen === "undone" ? copy.undoneTitle : copy.trashEmptyTitle }}</h1>
      <p class="center-body">{{ screen === "undone" ? copy.undoneBody : copy.trashEmptyBody }}</p>
      <div class="center-actions">
        <button class="btn btn-primary" type="button" @click="finish">{{ copy.finish }}</button>
      </div>
    </main>

    <!-- Suggestions List (Home) -->
    <template v-else>
      <main class="content">
        <div v-if="permissionMissing" class="notice">
          <AppIcon name="lock" :size="20" />
          <p>{{ copy.permission }}</p>
          <button class="btn btn-small" type="button" @click="openSettings">
            <AppIcon name="settings" :size="16" />{{ copy.openSettings }}
          </button>
        </div>

        <section v-if="hasSuggestions" class="hero">
          <div class="hero-text">
            <p class="eyebrow">{{ copy.freeableLabel }}</p>
            <p class="amount">
              <span class="num">{{ totalAmount.num }}</span>
              <span class="unit">{{ totalAmount.unit }}</span>
            </p>
            <p class="hero-sub">{{ copy.heroSub(allRows.length) }}</p>
          </div>
          <div class="disk">
            <div class="disk-labels">
              <span>{{ copy.diskLabel }}</span>
              <strong>{{ disk.free }}</strong>
            </div>
            <div class="meter" aria-hidden="true">
              <span class="meter-used" :style="{ width: `${disk.usedPct}%` }"></span>
              <span class="meter-freeing" :style="{ width: `${disk.freeingPct}%` }"></span>
            </div>
            <p class="disk-after">{{ copy.diskAfter(disk.after) }}</p>
          </div>
        </section>

        <section v-else class="empty">
          <span class="badge badge-ok"><AppIcon name="check" :size="34" /></span>
          <h1 class="center-title big">{{ copy.nothingTitle }}</h1>
          <p class="center-body">{{ copy.nothingBody }}</p>
          <button class="btn btn-secondary" type="button" @click="screen = 'ready'">{{ copy.reconfigureFolders }}</button>
        </section>

        <div v-if="hasSuggestions" class="list">
          <CleanSection
            v-for="section in sections"
            :key="section.kind"
            :section="section"
            :selected="selected"
            @toggle-row="toggleRow"
            @toggle-section="toggleSection"
            @reveal="reveal"
          />

          <section v-if="report.skipped.length" class="section kept">
            <div class="section-head static">
              <span class="tile tile-kept"><AppIcon name="lock" /></span>
              <span class="section-title">
                <span class="section-name">{{ copy.keptTitle }}</span>
                <span class="section-hint">{{ copy.keptHint }}</span>
              </span>
            </div>
            <ul class="rows">
              <li v-for="item in report.skipped" :key="item.path" class="row static">
                <span class="row-main">
                  <span class="row-name">{{ item.name }}</span>
                  <span class="row-meta">{{ copy.keptReason(placeOf(item.path)) }}</span>
                </span>
                <button class="reveal" type="button" :aria-label="copy.reveal" @click="reveal(item.path)">
                  <AppIcon name="search" :size="16" />
                </button>
              </li>
            </ul>
          </section>
        </div>
      </main>

      <footer v-if="hasSuggestions" class="actionbar">
        <div class="actionbar-text">
          <p class="actionbar-main">
            {{ pickedRows.length ? copy.footerPicked(pickedRows.length, pickedText) : copy.nothingPicked }}
          </p>
          <p class="actionbar-note">{{ copy.footerNote }}</p>
        </div>
        <button class="btn btn-primary btn-large" type="button" :disabled="!pickedRows.length" @click="openConfirm">
          <AppIcon name="trash" :size="18" />{{ copy.clean(pickedText) }}
        </button>
      </footer>
    </template>

    <!-- Confirm Dialog -->
    <div v-if="confirming" class="overlay" @click.self="confirming = false">
      <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="confirm-title">
        <span class="tile tile-confirm"><AppIcon name="trash" :size="22" /></span>
        <h2 id="confirm-title" class="dialog-title">{{ copy.confirmTitle(pickedRows.length, pickedText) }}</h2>
        <p class="dialog-body"><AppIcon name="undo" :size="16" />{{ copy.confirmUndo }}</p>
        <p class="dialog-body"><AppIcon name="lock" :size="16" />{{ copy.confirmSkip(report.skipped_sensitive_count) }}</p>
        <div class="dialog-actions">
          <button class="btn btn-secondary" type="button" @click="confirming = false">{{ copy.cancel }}</button>
          <button ref="confirmButton" class="btn btn-primary" type="button" @click="confirmClean">
            {{ copy.confirmButton }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
