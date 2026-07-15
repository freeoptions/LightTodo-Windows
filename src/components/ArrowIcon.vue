<script setup lang="ts">
import { computed } from 'vue';

const props = defineProps<{
  direction: 'up' | 'down';
}>();

const chevronPath = computed(() =>
  props.direction === 'up'
    ? 'M6 15L12 9L18 15'
    : 'M6 9L12 15L18 9'
);
</script>

<template>
  <svg
    class="arrow-icon"
    viewBox="0 0 24 24"
    aria-hidden="true"
    focusable="false"
  >
    <path
      :d="chevronPath"
      fill="none"
      stroke="currentColor"
      stroke-width="2.5"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
  </svg>
</template>

<style scoped>
.arrow-icon {
  display: block;
  width: 1em;
  height: 1em;
}
</style>
