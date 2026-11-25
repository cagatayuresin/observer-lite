export function formatApiError(error: unknown): string {
  const response = (error as { response?: { data?: { detail?: unknown }; status?: number } })?.response
  const detail = response?.data?.detail
  if (typeof detail === 'string' && detail.trim()) return detail
  if (Array.isArray(detail)) {
    const messages = detail
      .map((item) => {
        if (typeof item === 'string') return item
        if (item && typeof item === 'object' && 'msg' in item) return String((item as { msg: unknown }).msg)
        return ''
      })
      .filter(Boolean)
    if (messages.length) return messages.join(' ')
  }
  if (!response) return 'The server could not be reached.'
  return `Request failed (${response.status ?? 'error'}).`
}
