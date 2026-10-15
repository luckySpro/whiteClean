<script setup>
import AppIcon from "./AppIcon.vue";

defineProps({
  state: { type: String, default: "off" },
});
</script>

<template>
  <span class="box" :class="`box-${state}`" aria-hidden="true">
    <AppIcon v-if="state === 'on'" name="check" :size="14" />
    <AppIcon v-else-if="state === 'mixed'" name="minus" :size="14" />
  </span>
</template>
