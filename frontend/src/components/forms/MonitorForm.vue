<template>
  <form @submit.prevent="emit('submit')">
    <div class="space-y-8">
      <!-- Basic -->
      <section>
        <h2 class="text-lg font-bold text-white mb-4 border-b border-surface-700 pb-2">Basic Settings</h2>
        <div class="space-y-4">
          <div>
            <label class="label">Monitor Name *</label>
            <input v-model="form.name" class="input" placeholder="My API" required />
          </div>
          <div>
            <label class="label">URL / Host *</label>
            <input v-model="form.url" class="input" placeholder="https://example.com/api/health" required />
          </div>
          <div class="grid grid-cols-2 gap-4">
            <div>
              <label class="label">Type</label>
              <select v-model="form.monitor_type" class="input">
                <option value="http_get">HTTP GET</option>
                <option value="http_post">HTTP POST</option>
                <option value="http_head">HTTP HEAD</option>
                <option value="ping">PING</option>
                <option value="heartbeat">Heartbeat (push)</option>
              </select>
            </div>
            <div>
              <label class="label">Check Interval</label>
              <select v-model.number="form.check_interval_seconds" class="input">
                <option :value="30">30 seconds</option>
                <option :value="60">1 minute</option>
                <option :value="120">2 minutes</option>
                <option :value="300">5 minutes</option>
                <option :value="600">10 minutes</option>
                <option :value="1800">30 minutes</option>
                <option :value="3600">1 hour</option>
              </select>
            </div>
          </div>
          <div class="grid grid-cols-2 gap-4">
            <div>
              <label class="label">Timeout (seconds)</label>
              <input v-model.number="form.timeout_seconds" type="number" min="1" max="60" class="input" />
            </div>
            <div>
              <div class="flex items-center justify-between">
                <label class="label mb-0">Group</label>
                <button type="button" @click="$emit('request-new-group')" class="text-xs text-brand-400 hover:text-brand-300 font-medium">
                  + New Group
                </button>
              </div>
              <select v-model="form.group_id" class="input mt-1">
                <option :value="null">— No group —</option>
                <option v-for="g in groups" :key="g.id" :value="g.id">{{ g.name }}</option>
              </select>
            </div>
          </div>
          <div class="flex gap-6">
            <ToggleSwitch v-model="form.is_enabled" label="Enabled" />
            <ToggleSwitch v-model="form.alerts_enabled" label="Alerts enabled" />
          </div>
        </div>
      </section>

      <!-- Request -->
      <section>
        <h2 class="text-lg font-bold text-white mb-4 border-b border-surface-700 pb-2">Request & Assertions</h2>
        <div class="space-y-4">
          <div>
            <label class="label">Expected Status Codes</label>
            <input v-model="form.expected_status_codes" class="input font-mono" placeholder="2xx or 200|404 or !5xx" />
            <p class="text-xs text-slate-600 mt-1">Supports: <code>2xx</code>, <code>200</code>, <code>!5xx</code>, <code>200|404</code></p>
          </div>
          <div v-if="form.monitor_type !== 'ping' && form.monitor_type !== 'heartbeat'">
            <label class="label">Request Headers (JSON)</label>
            <textarea v-model="form.request_headers" class="input font-mono text-xs h-20" placeholder='{"Authorization": "Bearer token"}' />
          </div>
          <div v-if="form.monitor_type === 'http_post'">
            <label class="label">Request Body</label>
            <textarea v-model="form.request_body" class="input font-mono text-xs h-24" placeholder='{"key": "value"}' />
          </div>
          <div class="grid grid-cols-2 gap-4">
            <div>
              <label class="label">Response Body Match</label>
              <select v-model="form.expected_body_type" class="input">
                <option :value="null">— None —</option>
                <option value="contains">Contains</option>
                <option value="equals">Equals</option>
                <option value="not_equals">Not Equals</option>
              </select>
            </div>
            <div v-if="form.expected_body_type">
              <label class="label">Match Value</label>
              <input v-model="form.expected_body_value" class="input" placeholder="expected string" />
            </div>
          </div>
        </div>
      </section>

      <!-- Alerts -->
      <section>
        <h2 class="text-lg font-bold text-white mb-4 border-b border-surface-700 pb-2">Alerts</h2>
        <div class="space-y-4">
          <div class="grid grid-cols-2 gap-4">
            <div>
              <label class="label">Retries before DOWN</label>
              <input v-model.number="form.retry_count" type="number" min="1" max="10" class="input" />
              <p class="text-xs text-slate-600 mt-1">Consecutive failures before alert</p>
            </div>
            <div>
              <label class="label">Retry interval (seconds)</label>
              <input v-model.number="form.retry_interval_seconds" type="number" min="5" class="input" />
            </div>
          </div>
          <div class="grid grid-cols-2 gap-4">
            <div>
              <label class="label">Response time warning (ms)</label>
              <input v-model.number="form.response_time_warning_ms" type="number" min="100" class="input" />
            </div>
            <div>
              <label class="label">Alert cooldown (seconds)</label>
              <input v-model.number="form.alert_cooldown_seconds" type="number" min="60" class="input" />
            </div>
          </div>
        </div>
      </section>

      <!-- SSL -->
      <section>
        <h2 class="text-lg font-bold text-white mb-4 border-b border-surface-700 pb-2">SSL & Heartbeat</h2>
        <div class="space-y-4">
          <ToggleSwitch v-model="form.ssl_check_enabled" label="Enable SSL certificate monitoring" />
          <div v-if="form.ssl_check_enabled">
            <label class="label">Expiry warning (days before)</label>
            <input v-model.number="form.ssl_expiry_warning_days" type="number" min="1" max="90" class="input w-32" />
          </div>
          <div v-if="form.monitor_type === 'heartbeat'" class="mt-4">
            <label class="label">Heartbeat Grace Period (seconds)</label>
            <input v-model.number="form.heartbeat_grace_seconds" type="number" min="10" class="input w-32" />
            <p v-if="isEdit && monitorData?.heartbeat_token" class="text-xs text-slate-500 mt-2">
              Heartbeat URL: <code class="text-brand-400 break-all">/api/heartbeat/{{ monitorData.heartbeat_token }}</code>
            </p>
          </div>
        </div>
      </section>
    </div>

    <p v-if="error" class="text-red-400 text-sm mt-4">{{ error }}</p>

    <div class="flex gap-3 mt-8">
      <button type="submit" class="btn-primary" :disabled="saving">
        {{ saving ? 'Saving…' : (isEdit ? 'Save Changes' : 'Create Monitor') }}
      </button>
      <button type="button" class="btn-ghost" @click="emit('cancel')">Cancel</button>
    </div>
  </form>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import type { Monitor } from '@/stores/monitors'
import ToggleSwitch from '@/components/common/ToggleSwitch.vue'

interface FormModel {
  name: string
  url: string
  monitor_type: string
  group_id: number | null
  is_enabled: boolean
  check_interval_seconds: number
  timeout_seconds: number
  retry_count: number
  retry_interval_seconds: number
  alert_cooldown_seconds: number
  request_headers: string
  request_body: string
  expected_status_codes: string
  expected_body_type: string | null
  expected_body_value: string
  response_time_warning_ms: number
  ssl_check_enabled: boolean
  ssl_expiry_warning_days: number
  alerts_enabled: boolean
  heartbeat_grace_seconds: number
}

const props = defineProps<{
  modelValue: FormModel
  groups: { id: number; name: string }[]
  isEdit: boolean
  monitorData: Monitor | null
  saving: boolean
  error: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: FormModel]
  submit: []
  cancel: []
  'request-new-group': []
}>()

// Two-way binding: reflect parent changes into local proxy without infinite loops
const form = ref<FormModel>(JSON.parse(JSON.stringify(props.modelValue)))

watch(() => props.modelValue, (v) => {
  if (JSON.stringify(v) !== JSON.stringify(form.value)) {
    form.value = JSON.parse(JSON.stringify(v))
  }
}, { deep: true })

watch(form, (v) => {
  emit('update:modelValue', JSON.parse(JSON.stringify(v)))
}, { deep: true })
</script>
