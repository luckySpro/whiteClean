<script setup>
import { computed, ref } from "vue";
import { copy } from "../copy";
import { agoText, formatBytes, placeOf } from "../format";
import AppIcon from "./AppIcon.vue";
import CheckBox from "./CheckBox.vue";

const VISIBLE = 5;

const props = defineProps({
  section: { type: Object, required: true },
  selected: { type: Object, required: true },
});

const emit = defineEmits(["toggle-row", "toggle-section"]);

const open = ref(true);
const showAll = ref(false);

const meta = computed(() => copy.kinds[props.section.kind]);
const rows = computed(() => (showAll.value ? props.section.rows : props.section.rows.slice(0, VISIBLE)));
const hidden = computed(() => props.section.rows.length - rows.value.length);
const pickedCount = computed(() => props.section.rows.filter((row) => props.selected.has(row.path)).length);
const state = computed(() => {
  if (pickedCount.value === 0) return "off";
  return pickedCount.value === props.section.rows.length ? "on" : "mixed";
});
const pickedBytes = computed(() =>
  props.section.rows.filter((row) => props.selected.has(row.path)).reduce((sum, row) => sum + row.size, 0),
);

function detail(row) {
  const place = placeOf(row.path);
  switch (row.kind) {
    case "exact_duplicate":
      return copy.dupDetail(place, placeOf(row.keep_path), row.keep_name);
    case "older_version":
      return copy.versionDetail(row.keep_name, row.note === "content_differs");
    case "old_installer":
      return copy.installerDetail(place, agoText(row.modified_ms));
    default:
      return row.hint || "";
  }
}
</script>

<template>
  <section class="section">
    <div class="section-head">
      <button
        class="section-check"
        type="button"
        role="checkbox"
        :aria-checked="state === 'mixed' ? 'mixed' : state === 'on'"
        :aria-label="copy.selectAll(meta.title)"
        @click="emit('toggle-section', section)"
      >
        <CheckBox :state="state" />
      </button>
      <button class="section-toggle" type="button" :aria-expanded="open" @click="open = !open">
        <span class="tile" :class="`tile-${section.kind}`">
          <AppIcon :name="meta.icon" />
        </span>
        <span class="section-title">
          <span class="section-name">{{ meta.title }}</span>
          <span class="section-hint">{{ meta.hint }}</span>
        </span>
        <span class="section-size">
          <span class="section-bytes">{{ formatBytes(pickedBytes).text }}</span>
          <span class="section-count">{{ copy.pickedOf(pickedCount, section.rows.length) }}</span>
        </span>
        <span class="chevron" :class="{ open }"><AppIcon name="chevron" :size="16" /></span>
      </button>
    </div>

    <ul v-if="open" class="rows">
      <li v-for="row in rows" :key="row.path">
        <button
          class="row"
          :class="{ off: !selected.has(row.path) }"
          type="button"
          role="checkbox"
          :aria-checked="selected.has(row.path)"
          @click="emit('toggle-row', row.path)"
        >
          <CheckBox :state="selected.has(row.path) ? 'on' : 'off'" />
          <span class="row-main">
            <span class="row-name">{{ row.label || row.name }}</span>
            <span v-if="detail(row)" class="row-meta">{{ detail(row) }}</span>
          </span>
          <span class="row-size">{{ formatBytes(row.size).text }}</span>
        </button>
      </li>
      <li v-if="hidden > 0 || showAll">
        <button class="more" type="button" @click="showAll = !showAll">
          {{ showAll ? copy.showLess : copy.showMore(hidden) }}
        </button>
      </li>
    </ul>
  </section>
</template>
