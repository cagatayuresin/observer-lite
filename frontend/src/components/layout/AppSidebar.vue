<template>
  <aside class="w-56 shrink-0 bg-surface-800 border-r border-surface-700 flex flex-col">
    <!-- Logo -->
    <div class="px-4 py-5 border-b border-surface-700">
      <div class="flex items-center gap-2">
        <svg class="w-7 h-7 text-brand-500" fill="currentColor" viewBox="0 0 24 24">
          <circle cx="12" cy="12" r="10" opacity=".15"/>
          <circle cx="12" cy="12" r="6" opacity=".3"/>
          <circle cx="12" cy="12" r="2.5"/>
        </svg>
        <span class="font-semibold text-white text-sm">Observer Lite</span>
      </div>
    </div>

    <!-- Nav -->
    <nav class="flex-1 px-2 py-4 space-y-0.5 overflow-y-auto">
      <NavItem to="/dashboard" icon="grid">Dashboard</NavItem>
      <NavItem to="/incidents" icon="alert">Incidents</NavItem>
      <NavItem to="/groups" icon="folder">Groups</NavItem>
      <NavItem to="/maintenance" icon="wrench">Maintenance</NavItem>
      <NavItem to="/notifications" icon="bell">Channels</NavItem>
      <div class="pt-4 pb-1 px-2 text-xs font-semibold text-slate-500 uppercase tracking-wider">Admin</div>
      <NavItem v-if="auth.isSuperAdmin" to="/users" icon="users">Users</NavItem>
      <NavItem v-if="auth.isAdmin" to="/api-keys" icon="key">API Keys</NavItem>
      <NavItem v-if="auth.isSuperAdmin" to="/audit-log" icon="list">Audit Log</NavItem>
      <NavItem v-if="auth.isSuperAdmin" to="/settings" icon="settings">Settings</NavItem>
    </nav>

    <!-- User -->
    <div class="border-t border-surface-700 px-3 py-3">
      <div class="flex items-center gap-2">
        <div class="w-7 h-7 rounded-full bg-brand-600 flex items-center justify-center text-xs font-bold text-white">
          {{ auth.user?.username?.[0]?.toUpperCase() }}
        </div>
        <div class="flex-1 min-w-0">
          <p class="text-xs font-medium text-white truncate">{{ auth.user?.username }}</p>
          <p class="text-xs text-slate-500 capitalize">{{ auth.user?.role }}</p>
        </div>
        <button @click="handleLogout" class="text-slate-500 hover:text-white transition-colors" title="Logout">
          <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1" />
          </svg>
        </button>
      </div>
    </div>
  </aside>
</template>

<script setup lang="ts">
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import NavItem from './NavItem.vue'

const auth = useAuthStore()
const router = useRouter()

function handleLogout() {
  auth.logout()
  router.push('/login')
}
</script>
