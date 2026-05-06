import { api } from './client'

export const monitorsApi = {
  list: (params?: { status?: string; group_id?: number; search?: string }) =>
    api.get('/monitors', { params }),

  get: (id: number) => api.get(`/monitors/${id}`),

  create: (data: Record<string, unknown>) => api.post('/monitors', data),

  update: (id: number, data: Record<string, unknown>) => api.put(`/monitors/${id}`, data),

  patch: (id: number, data: Record<string, unknown>) => api.patch(`/monitors/${id}`, data),

  delete: (id: number) => api.delete(`/monitors/${id}`),

  pause: (id: number) => api.post(`/monitors/${id}/pause`),

  resume: (id: number) => api.post(`/monitors/${id}/resume`),

  checkNow: (id: number) => api.post(`/monitors/${id}/check-now`),

  bulk: (action: string, ids: number[]) => api.post('/monitors/bulk', { action, ids }),

  stats: (id: number, days = 30) => api.get(`/monitors/${id}/stats`, { params: { days } }),

  history: (id: number, params?: { from?: string; to?: string; limit?: number }) =>
    api.get('/check-results', { params: { monitor_id: id, ...params } }),

  incidents: (id: number) => api.get('/incidents', { params: { monitor_id: id } }),

  getUsers: (id: number) => api.get(`/monitors/${id}/users`),

  assignUser: (id: number, user_id: number, notify = true) =>
    api.post(`/monitors/${id}/users`, null, { params: { user_id, notify } }),

  removeUser: (id: number, user_id: number) => api.delete(`/monitors/${id}/users/${user_id}`),

  getChannels: (id: number) => api.get(`/monitors/${id}/channels`),

  assignChannel: (id: number, data: { channel_id: number; on_down: boolean; on_recovery: boolean; on_ssl_warn: boolean }) =>
    api.post(`/monitors/${id}/channels`, data),

  removeChannel: (id: number, channel_id: number) =>
    api.delete(`/monitors/${id}/channels/${channel_id}`),
}
