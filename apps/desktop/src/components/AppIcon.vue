<script setup>
import { computed } from "vue";

const props = defineProps({
  name: { type: String, required: true },
  size: { type: Number, default: 20 },
});

const PATHS = {
  copy: [
    "M9 9h10a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H9a1 1 0 0 1-1-1V10a1 1 0 0 1 1-1z",
    "M16 9V5a1 1 0 0 0-1-1H5a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h3",
  ],
  layers: ["M12 3.5l8.5 4.5-8.5 4.5L3.5 8z", "M3.5 12.5l8.5 4.5 8.5-4.5", "M3.5 16.5L12 21l8.5-4.5"],
  box: ["M20.5 7.5L12 3 3.5 7.5v9L12 21l8.5-4.5z", "M3.5 7.5L12 12l8.5-4.5", "M12 12v9"],
  wind: ["M3 8.5h11a3 3 0 1 0-3-3", "M3 12.5h15a3 3 0 1 1-3 3", "M3 16.5h6"],
  lock: ["M6 11h12a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1v-8a1 1 0 0 1 1-1z", "M8 11V7.5a4 4 0 0 1 8 0V11"],
  check: ["M5 12.5l4.5 4.5L19 7.5"],
  minus: ["M6.5 12h11"],
  chevron: ["M9 6l6 6-6 6"],
  shield: ["M12 3l7.5 3v5.6c0 4.4-3.1 8.2-7.5 9.4-4.4-1.2-7.5-5-7.5-9.4V6z", "M8.8 12.2l2.3 2.3 4.2-4.6"],
  trash: ["M4 7h16", "M9.5 7V4.5h5V7", "M6 7l1 13h10l1-13", "M10 11v5.5", "M14 11v5.5"],
  undo: ["M9 14L4 9l5-5", "M4 9h10.5a5.5 5.5 0 0 1 0 11H11"],
  settings: ["M12 9a3 3 0 1 0 0 6 3 3 0 0 0 0-6z", "M19 12a7 7 0 0 0-.1-1.2l2-1.6-2-3.4-2.4 1a7 7 0 0 0-2-1.2L14 3h-4l-.5 2.6a7 7 0 0 0-2 1.2l-2.4-1-2 3.4 2 1.6a7 7 0 0 0 0 2.4l-2 1.6 2 3.4 2.4-1a7 7 0 0 0 2 1.2L10 21h4l.5-2.6a7 7 0 0 0 2-1.2l2.4 1 2-3.4-2-1.6c.1-.4.1-.8.1-1.2z"],
  search: ["M11 5.5a5.5 5.5 0 1 0 0.01 0", "M15.2 15.2L20.5 20.5"],
  folder: ["M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2z"],
  plus: ["M12 5v14", "M5 12h14"],
  x: ["M18 6L6 18", "M6 6l12 12"],
  play: ["M5 3l14 9-14 9V3z"],
};

const paths = computed(() => PATHS[props.name] || []);
</script>

<template>
  <svg
    class="icon"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.8"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path v-for="d in paths" :key="d" :d="d" />
  </svg>
</template>
