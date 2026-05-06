<template>
  <div class="space-y-4">
    <div>
      <label class="label">Name</label>
      <input v-model="form.name" class="input" placeholder="My Email Channel" required />
    </div>

    <div>
      <label class="label">Type</label>
      <select v-model="form.type" class="input">
        <option value="email">Email</option>
        <option value="telegram">Telegram</option>
      </select>
    </div>

    <!-- Email config -->
    <template v-if="form.type === 'email'">
      <div class="grid grid-cols-2 gap-3">
        <div>
          <label class="label">SMTP Host</label>
          <input v-model="form.config.smtp_host" class="input" placeholder="smtp.gmail.com" />
        </div>
        <div>
          <label class="label">SMTP Port</label>
          <input v-model.number="form.config.smtp_port" type="number" class="input" placeholder="587" />
        </div>
      </div>
      <div class="grid grid-cols-2 gap-3">
        <div>
          <label class="label">Username</label>
          <input v-model="form.config.smtp_user" class="input" />
        </div>
        <div>
          <label class="label">Password</label>
          <input v-model="form.config.smtp_password_enc" type="password" class="input" />
        </div>
      </div>
      <div>
        <label class="label">From Address</label>
        <input v-model="form.config.smtp_from" class="input" placeholder="alerts@example.com" />
      </div>
      <div>
        <label class="label">Recipients (comma-separated)</label>
        <input v-model="form.config.recipients_str" class="input" placeholder="ops@example.com, dev@example.com" />
      </div>
      <label class="flex items-center gap-2 cursor-pointer">
        <input type="checkbox" v-model="form.config.use_tls" class="accent-brand-500" />
        <span class="text-sm text-slate-300">Use TLS</span>
      </label>
    </template>

    <!-- Telegram config -->
    <template v-else>
      <div>
        <label class="label">Bot Token</label>
        <input v-model="form.config.bot_token" class="input" placeholder="123456:ABC-DEF..." />
      </div>
      <div>
        <label class="label">Chat ID</label>
        <input v-model="form.config.chat_id" class="input" placeholder="-1001234567890" />
      </div>
    </template>

    <label class="flex items-center gap-2 cursor-pointer">
      <input type="checkbox" v-model="form.is_enabled" class="accent-brand-500" />
      <span class="text-sm text-slate-300">Enabled</span>
    </label>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'

interface ConfigModel {
  smtp_host: string
  smtp_port: number
  smtp_user: string
  smtp_password_enc: string
  smtp_from: string
  recipients_str: string
  use_tls: boolean
  bot_token: string
  chat_id: string
}

interface ChannelFormModel {
  name: string
  type: string
  is_enabled: boolean
  config: ConfigModel
}

const props = defineProps<{
  modelValue: ChannelFormModel
}>()

const emit = defineEmits<{
  'update:modelValue': [value: ChannelFormModel]
}>()

const form = ref<ChannelFormModel>(structuredClone(props.modelValue))

// Sync inward only when the parent replaces the whole object (modal open/reset),
// not on every keystroke. Guard prevents the echo loop: emit → parent sets prop
// → this watcher fires → emit again → …
let ignoreNext = false
watch(() => props.modelValue, (v) => {
  if (ignoreNext) { ignoreNext = false; return }
  form.value = structuredClone(v)
})

watch(form, (v) => { ignoreNext = true; emit('update:modelValue', v) }, { deep: true })
</script>
