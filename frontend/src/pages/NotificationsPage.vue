<template>
  <div class="p-6 max-w-3xl mx-auto">
    <div class="flex items-center justify-between mb-6">
      <h1 class="text-xl font-bold text-white">Notification Channels</h1>
      <button class="btn-primary" @click="openForm()">+ Add Channel</button>
    </div>

    <div v-if="!channels.length" class="text-slate-500 text-sm text-center py-12">No channels yet.</div>
    <div v-else class="card divide-y divide-surface-700">
      <div v-for="ch in channels" :key="ch.id" class="flex items-center gap-4 px-4 py-3">
        <div class="flex-1">
          <p class="text-sm font-medium text-white">{{ ch.name }}</p>
          <p class="text-xs text-slate-500 capitalize">{{ ch.channel_type }}</p>
        </div>
        <span :class="ch.is_enabled ? 'badge-up' : 'badge-paused'">{{ ch.is_enabled ? 'Active' : 'Disabled' }}</span>
        <button class="btn-ghost text-xs" @click="testChannel(ch.id)">Test</button>
        <button class="btn-ghost text-xs" @click="openForm(ch)">Edit</button>
        <button class="btn-ghost text-xs text-red-400" @click="deleteChannel(ch.id)">Delete</button>
      </div>
    </div>

    <!-- Modal -->
    <Teleport to="body">
      <div v-if="showForm" class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60">
        <div class="card w-full max-w-lg p-6">
          <h2 class="text-lg font-semibold text-white mb-4">{{ editing ? 'Edit Channel' : 'New Channel' }}</h2>

          <NotificationChannelForm v-model="form" />

          <div class="flex gap-3 mt-6">
            <button class="btn-primary" @click="saveChannel">{{ editing ? 'Save' : 'Create' }}</button>
            <button class="btn-ghost" @click="showForm = false">Cancel</button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { channelsApi } from '@/api/index'
import { useToastStore } from '@/stores/toast'
import NotificationChannelForm from '@/components/forms/NotificationChannelForm.vue'

const toast = useToastStore()
const channels = ref<{ id: number; name: string; channel_type: string; is_enabled: boolean }[]>([])
const showForm = ref(false)
const editing = ref<number | null>(null)

const defaultForm = () => ({
  name: '',
  type: 'email',
  is_enabled: true,
  config: {
    smtp_host: '',
    smtp_port: 587,
    smtp_user: '',
    smtp_password_enc: '',
    smtp_from: '',
    recipients_str: '',
    use_tls: true,
    bot_token: '',
    chat_id: '',
  },
})

const form = ref(defaultForm())

onMounted(load)

async function load() {
  const { data } = await channelsApi.list()
  channels.value = data
}

function openForm(ch?: typeof channels.value[number]) {
  editing.value = ch?.id ?? null
  form.value = { ...defaultForm(), name: ch?.name ?? '', type: ch?.channel_type ?? 'email', is_enabled: ch?.is_enabled ?? true }
  showForm.value = true
}

async function saveChannel() {
  const cfg = { ...form.value.config }
  if (form.value.type === 'email') {
    (cfg as Record<string, unknown>).recipients = form.value.config.recipients_str
      .split(',')
      .map((s) => s.trim())
      .filter(Boolean)
  }
  const payload = { name: form.value.name, channel_type: form.value.type, config: cfg, is_enabled: form.value.is_enabled }
  if (editing.value) {
    await channelsApi.update(editing.value, payload)
    toast.success('Channel updated')
  } else {
    await channelsApi.create(payload)
    toast.success('Channel created')
  }
  showForm.value = false
  await load()
}

async function deleteChannel(id: number) {
  await channelsApi.delete(id)
  channels.value = channels.value.filter((c) => c.id !== id)
  toast.success('Channel deleted')
}

async function testChannel(id: number) {
  try {
    await channelsApi.test(id)
    toast.success('Test notification sent!')
  } catch {
    toast.error('Test failed — check configuration')
  }
}
</script>
