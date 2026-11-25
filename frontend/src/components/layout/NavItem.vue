<template>
  <RouterLink
    :to="to"
    class="flex items-center gap-2.5 rounded-lg px-3 py-2 text-sm font-medium transition-colors"
    :class="isActive ? 'bg-brand-600/20 text-brand-400' : 'text-mist hover:bg-surface-700 hover:text-cream'"
    @click="$emit('navigate')"
  >
    <component :is="iconComponent" class="h-4 w-4 shrink-0" />
    <slot />
  </RouterLink>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { RouterLink, useRoute } from 'vue-router'
import {
  IconAlertTriangle,
  IconBell,
  IconFolder,
  IconKey,
  IconLayoutDashboard,
  IconList,
  IconSettings,
  IconTool,
  IconUsers,
} from '@tabler/icons-vue'

const props = defineProps<{ to: string; icon: string }>()
defineEmits<{ navigate: [] }>()
const route = useRoute()
const isActive = computed(() => route.path === props.to || (props.to !== '/dashboard' && route.path.startsWith(props.to)))

const icons: Record<string, unknown> = {
  grid: IconLayoutDashboard,
  alert: IconAlertTriangle,
  folder: IconFolder,
  wrench: IconTool,
  bell: IconBell,
  users: IconUsers,
  key: IconKey,
  list: IconList,
  settings: IconSettings,
}

const iconComponent = computed(() => icons[props.icon] ?? IconLayoutDashboard)
</script>
