<template>
  <aside
    class="fixed inset-y-0 left-0 z-40 flex w-64 shrink-0 -translate-x-full flex-col border-r border-mist/30 bg-surface-800 transition-transform md:static md:translate-x-0"
    :class="open ? 'translate-x-0' : '-translate-x-full'"
  >
    <div class="flex items-center gap-2 border-b border-mist/30 px-4 py-5">
      <svg class="h-7 w-7 text-brand-500" fill="currentColor" viewBox="0 0 24 24" aria-hidden="true">
        <circle cx="12" cy="12" r="10" opacity=".15"/>
        <circle cx="12" cy="12" r="6" opacity=".3"/>
        <circle cx="12" cy="12" r="2.5"/>
      </svg>
      <span class="text-sm font-semibold text-cream">Observer Lite</span>
      <button type="button" class="ml-auto rounded p-1 text-mist md:hidden" aria-label="Close menu" @click="$emit('close')">
        <IconX class="h-4 w-4" />
      </button>
    </div>

    <nav class="flex-1 space-y-0.5 overflow-y-auto px-2 py-4">
      <NavItem to="/dashboard" icon="grid" @navigate="$emit('navigate')">Dashboard</NavItem>
      <NavItem to="/incidents" icon="alert" @navigate="$emit('navigate')">Incidents</NavItem>
      <NavItem to="/groups" icon="folder" @navigate="$emit('navigate')">Groups</NavItem>
      <NavItem to="/maintenance" icon="wrench" @navigate="$emit('navigate')">Maintenance</NavItem>
      <NavItem to="/notifications" icon="bell" @navigate="$emit('navigate')">Channels</NavItem>
      <div class="px-2 pb-1 pt-4 text-xs font-semibold uppercase tracking-wider text-mist">Admin</div>
      <NavItem v-if="auth.isSuperAdmin" to="/users" icon="users" @navigate="$emit('navigate')">Users</NavItem>
      <NavItem v-if="auth.isAdmin" to="/api-keys" icon="key" @navigate="$emit('navigate')">API Keys</NavItem>
      <NavItem v-if="auth.isSuperAdmin" to="/audit-log" icon="list" @navigate="$emit('navigate')">Audit Log</NavItem>
      <NavItem v-if="auth.isSuperAdmin" to="/settings" icon="settings" @navigate="$emit('navigate')">Settings</NavItem>
    </nav>

    <div class="border-t border-mist/30 px-3 py-3">
      <div class="flex items-center gap-2">
        <div class="flex h-7 w-7 items-center justify-center rounded-full bg-brand-600 text-xs font-bold text-surface-900">
          {{ auth.user?.username?.[0]?.toUpperCase() }}
        </div>
        <div class="min-w-0 flex-1">
          <p class="truncate text-xs font-medium text-cream">{{ auth.user?.username }}</p>
          <p class="text-xs capitalize text-mist">{{ auth.user?.role }}</p>
        </div>
        <button class="text-mist transition-colors hover:text-cream" title="Logout" type="button" @click="handleLogout">
          <IconLogout class="h-4 w-4" />
        </button>
      </div>
    </div>
  </aside>
</template>

<script setup lang="ts">
import { useRouter } from 'vue-router'
import { IconLogout, IconX } from '@tabler/icons-vue'
import { useAuthStore } from '@/stores/auth'
import NavItem from './NavItem.vue'

defineProps<{ open: boolean }>()
defineEmits<{ close: []; navigate: [] }>()

const auth = useAuthStore()
const router = useRouter()

function handleLogout() {
  auth.logout()
  router.push('/login')
}
</script>
