<template>
  <div class="p-6 max-w-2xl mx-auto">
    <div class="flex items-center gap-3 mb-6">
      <RouterLink to="/dashboard" class="btn-ghost px-2 py-1">← Back</RouterLink>
      <h1 class="text-xl font-bold text-white">{{ isEdit ? 'Edit Monitor' : 'New Monitor' }}</h1>
    </div>

    <MonitorForm
      v-model="form"
      :groups="groups"
      :is-edit="isEdit"
      :monitor-data="monitorData"
      :saving="saving"
      :error="error"
      @submit="handleSubmit"
      @cancel="router.push('/dashboard')"
    />
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref, computed } from 'vue'
import { RouterLink, useRoute, useRouter } from 'vue-router'
import { monitorsApi } from '@/api/monitors'
import { groupsApi } from '@/api/index'
import { useToastStore } from '@/stores/toast'
import type { Monitor } from '@/stores/monitors'
import MonitorForm from '@/components/forms/MonitorForm.vue'

const route = useRoute()
const router = useRouter()
const toast = useToastStore()

const isEdit = computed(() => !!route.params.id)
const monitorId = computed(() => Number(route.params.id))
const monitorData = ref<Monitor | null>(null)
const saving = ref(false)
const error = ref('')
const groups = ref<{ id: number; name: string }[]>([])

const form = ref({
  name: '',
  url: '',
  monitor_type: 'http_get',
  group_id: null as number | null,
  is_enabled: true,
  check_interval_seconds: 60,
  timeout_seconds: 10,
  retry_count: 3,
  retry_interval_seconds: 20,
  alert_cooldown_seconds: 1800,
  request_headers: '',
  request_body: '',
  expected_status_codes: '2xx',
  expected_body_type: null as string | null,
  expected_body_value: '',
  response_time_warning_ms: 2000,
  ssl_check_enabled: true,
  ssl_expiry_warning_days: 30,
  alerts_enabled: true,
  heartbeat_grace_seconds: 60,
})

onMounted(async () => {
  const [{ data: grps }] = await Promise.all([groupsApi.list()])
  groups.value = grps
  if (isEdit.value) {
    const { data } = await monitorsApi.get(monitorId.value)
    monitorData.value = data
    Object.assign(form.value, {
      name: data.name,
      url: data.url,
      monitor_type: data.monitor_type,
      group_id: data.group_id,
      is_enabled: data.is_enabled,
      check_interval_seconds: data.check_interval_seconds,
      timeout_seconds: data.timeout_seconds,
      retry_count: data.retry_count,
      retry_interval_seconds: data.retry_interval_seconds,
      alert_cooldown_seconds: data.alert_cooldown_seconds,
      request_headers: data.request_headers ?? '',
      request_body: data.request_body ?? '',
      expected_status_codes: data.expected_status_codes,
      expected_body_type: data.expected_body_type,
      expected_body_value: data.expected_body_value ?? '',
      response_time_warning_ms: data.response_time_warning_ms,
      ssl_check_enabled: data.ssl_check_enabled,
      ssl_expiry_warning_days: data.ssl_expiry_warning_days,
      alerts_enabled: data.alerts_enabled,
      heartbeat_grace_seconds: data.heartbeat_grace_seconds,
    })
  }
})

async function handleSubmit() {
  saving.value = true
  error.value = ''
  try {
    const payload = {
      ...form.value,
      request_headers: form.value.request_headers || null,
      request_body: form.value.request_body || null,
      expected_body_type: form.value.expected_body_type || null,
      expected_body_value: form.value.expected_body_value || null,
    }
    if (isEdit.value) {
      await monitorsApi.update(monitorId.value, payload)
      toast.success('Monitor updated')
    } else {
      await monitorsApi.create(payload)
      toast.success('Monitor created')
    }
    router.push('/dashboard')
  } catch (e: unknown) {
    const detail = (e as { response?: { data?: { detail?: unknown } } })?.response?.data?.detail
    error.value = typeof detail === 'string' ? detail : JSON.stringify(detail) ?? 'Save failed'
  } finally {
    saving.value = false
  }
}
</script>
