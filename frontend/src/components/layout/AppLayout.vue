<template>
  <div class="flex h-screen overflow-hidden bg-surface-900 text-cream">
    <div v-if="open" class="fixed inset-0 z-30 bg-black/50 md:hidden" @click="open = false" />
    <AppSidebar :open="open" @close="open = false" @navigate="open = false" />
    <div class="flex min-w-0 flex-1 flex-col">
      <header class="flex h-14 items-center gap-3 border-b border-mist/30 px-4 md:hidden">
        <button type="button" class="rounded-lg p-2 text-mist hover:bg-surface-700 hover:text-cream" aria-label="Open menu" @click="open = true">
          <IconMenu2 class="h-5 w-5" />
        </button>
        <span class="text-sm font-semibold">Observer Lite</span>
      </header>
      <main class="flex-1 overflow-y-auto">
        <RouterView />
      </main>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from 'vue'
import { RouterView, useRoute } from 'vue-router'
import { IconMenu2 } from '@tabler/icons-vue'
import AppSidebar from './AppSidebar.vue'
import { useSSEStore } from '@/stores/sse'

const open = ref(false)
const route = useRoute()
watch(() => route.fullPath, () => { open.value = false })

const sse = useSSEStore()
onMounted(() => sse.connect())
onUnmounted(() => sse.disconnect())
</script>
