import { defineStore } from 'pinia'
import { ref } from 'vue'
import { monitorsApi } from '@/api/monitors'

export interface Monitor {
  id: number
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
  request_headers: string | null
  request_body: string | null
  expected_status_codes: string
  expected_body_type: string | null
  expected_body_value: string | null
  response_time_warning_ms: number
  ssl_check_enabled: boolean
  ssl_expiry_warning_days: number
  alerts_enabled: boolean
  heartbeat_token: string | null
  heartbeat_grace_seconds: number
  current_status: string
  last_checked_at: string | null
  last_response_time_ms: number | null
  consecutive_failures: number
  created_at: string
  updated_at: string
}

export const useMonitorStore = defineStore('monitors', () => {
  const monitors = ref<Monitor[]>([])
  const loading = ref(false)
  const search = ref('')
  const statusFilter = ref('')
  const groupFilter = ref<number | null>(null)

  async function fetchMonitors() {
    loading.value = true
    try {
      const { data } = await monitorsApi.list({
        status: statusFilter.value || undefined,
        search: search.value || undefined,
        group_id: groupFilter.value || undefined,
      })
      monitors.value = data
    } finally {
      loading.value = false
    }
  }

  function updateFromSSE(event: { monitor_id: number; status: string; response_time_ms: number | null; checked_at: string }) {
    const idx = monitors.value.findIndex((m) => m.id === event.monitor_id)
    if (idx !== -1) {
      monitors.value[idx].current_status = event.status
      monitors.value[idx].last_response_time_ms = event.response_time_ms
      monitors.value[idx].last_checked_at = event.checked_at
    }
  }

  return { monitors, loading, search, statusFilter, groupFilter, fetchMonitors, updateFromSSE }
})
