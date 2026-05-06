<template>
  <div class="p-6 max-w-2xl mx-auto">
    <h1 class="text-xl font-bold text-white mb-6">Settings</h1>

    <div class="card p-6 space-y-6">
      <div>
        <h2 class="text-sm font-semibold text-white mb-4">General</h2>
        <div class="space-y-3">
          <div>
            <label class="label">Application Base URL</label>
            <input v-model="settings.app_base_url" class="input" placeholder="https://monitor.example.com" />
          </div>
          <div>
            <label class="label">Data Retention (days)</label>
            <input v-model.number="settings.data_retention_days" type="number" min="7" max="365" class="input w-32" />
          </div>
        </div>
      </div>

      <div>
        <h2 class="text-sm font-semibold text-white mb-4">SMTP (Global)</h2>
        <div class="grid grid-cols-2 gap-3">
          <div>
            <label class="label">Host</label>
            <input v-model="settings.smtp_host" class="input" placeholder="smtp.gmail.com" />
          </div>
          <div>
            <label class="label">Port</label>
            <input v-model.number="settings.smtp_port" type="number" class="input" placeholder="587" />
          </div>
          <div>
            <label class="label">Username</label>
            <input v-model="settings.smtp_user" class="input" />
          </div>
          <div>
            <label class="label">Password</label>
            <input v-model="settings.smtp_password_enc" type="password" class="input" placeholder="(encrypted)" />
          </div>
          <div>
            <label class="label">From Address</label>
            <input v-model="settings.smtp_from" class="input" placeholder="alerts@example.com" />
          </div>
        </div>
        <button class="btn-ghost text-xs mt-3" @click="testSmtp">Test SMTP</button>
      </div>

      <div class="flex gap-3 pt-2">
        <button class="btn-primary" @click="save" :disabled="saving">{{ saving ? 'Saving…' : 'Save Settings' }}</button>
      </div>
      <p v-if="msg" class="text-sm" :class="msg.type === 'ok' ? 'text-emerald-400' : 'text-red-400'">{{ msg.text }}</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { settingsApi } from '@/api/index'

const settings = ref<Record<string, string | number>>({
  app_base_url: '',
  data_retention_days: 90,
  smtp_host: '',
  smtp_port: 587,
  smtp_user: '',
  smtp_password_enc: '',
  smtp_from: '',
})
const saving = ref(false)
const msg = ref<{ type: 'ok' | 'err'; text: string } | null>(null)

onMounted(async () => {
  const { data } = await settingsApi.get()
  Object.assign(settings.value, data)
})

async function save() {
  saving.value = true
  msg.value = null
  try {
    await settingsApi.update(settings.value)
    msg.value = { type: 'ok', text: 'Settings saved.' }
  } catch {
    msg.value = { type: 'err', text: 'Save failed.' }
  } finally {
    saving.value = false
  }
}

async function testSmtp() {
  try {
    await settingsApi.testSmtp()
    msg.value = { type: 'ok', text: 'SMTP test email sent!' }
  } catch {
    msg.value = { type: 'err', text: 'SMTP test failed — check settings.' }
  }
}
</script>
