import { defineStore } from 'pinia'
import { useMonitorStore } from './monitors'

export const useSSEStore = defineStore('sse', () => {
  let es: EventSource | null = null

  function connect() {
    if (es) return
    const token = localStorage.getItem('access_token')
    if (!token) return

    // SSE endpoint uses bearer auth via query param workaround
    // (EventSource doesn't support custom headers natively)
    es = new EventSource(`/api/sse/dashboard?token=${token}`)

    es.addEventListener('monitor.check_result', (e) => {
      const data = JSON.parse(e.data)
      useMonitorStore().updateFromSSE(data)
    })

    es.onerror = () => {
      es?.close()
      es = null
      // Reconnect after 5s
      setTimeout(connect, 5000)
    }
  }

  function disconnect() {
    es?.close()
    es = null
  }

  return { connect, disconnect }
})
