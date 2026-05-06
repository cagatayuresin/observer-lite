<template>
  <div class="flex h-screen overflow-hidden bg-surface-900">
    <AppSidebar />
    <main class="flex-1 overflow-y-auto">
      <RouterView />
    </main>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import { RouterView } from 'vue-router'
import AppSidebar from './AppSidebar.vue'
import { useSSEStore } from '@/stores/sse'

const sse = useSSEStore()
onMounted(() => sse.connect())
onUnmounted(() => sse.disconnect())
</script>
