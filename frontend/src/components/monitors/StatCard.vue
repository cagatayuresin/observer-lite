<template>
  <div class="card px-4 py-3 flex items-center gap-3">
    <div class="w-2 h-8 rounded-full" :class="`bg-${color}-500`" />
    <div>
      <p class="text-2xl font-bold text-white">{{ value }}</p>
      <p class="text-xs text-slate-500">{{ label }}</p>
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{ label: string; value: number; color: string }>()
</script>
