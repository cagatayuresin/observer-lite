import { api } from './client'

export { authApi } from './auth'
export { monitorsApi } from './monitors'

export const usersApi = {
  list: () => api.get('/users'),
  create: (data: Record<string, unknown>) => api.post('/users', data),
  update: (id: number, data: Record<string, unknown>) => api.put(`/users/${id}`, data),
  delete: (id: number) => api.delete(`/users/${id}`),
  resetPassword: (id: number) => api.post(`/users/${id}/reset-password`),
}

export const channelsApi = {
  list: () => api.get('/channels'),
  create: (data: Record<string, unknown>) => api.post('/channels', data),
  update: (id: number, data: Record<string, unknown>) => api.put(`/channels/${id}`, data),
  delete: (id: number) => api.delete(`/channels/${id}`),
  test: (id: number) => api.post(`/channels/${id}/test`),
}

export const groupsApi = {
  list: () => api.get('/groups'),
  create: (data: Record<string, unknown>) => api.post('/groups', data),
  update: (id: number, data: Record<string, unknown>) => api.put(`/groups/${id}`, data),
  delete: (id: number) => api.delete(`/groups/${id}`),
}

export const incidentsApi = {
  list: (params?: Record<string, unknown>) => api.get('/incidents', { params }),
  acknowledge: (id: number) => api.post(`/incidents/${id}/acknowledge`),
}

export const maintenanceApi = {
  list: () => api.get('/maintenance'),
  create: (data: Record<string, unknown>) => api.post('/maintenance', data),
  update: (id: number, data: Record<string, unknown>) => api.put(`/maintenance/${id}`, data),
  delete: (id: number) => api.delete(`/maintenance/${id}`),
}

export const apiKeysApi = {
  list: () => api.get('/api-keys'),
  create: (data: Record<string, unknown>) => api.post('/api-keys', data),
  delete: (id: number) => api.delete(`/api-keys/${id}`),
}

export const settingsApi = {
  get: () => api.get('/settings'),
  update: (data: Record<string, unknown>) => api.put('/settings', data),
  testSmtp: () => api.post('/settings/test-smtp'),
}

export const auditLogApi = {
  list: (params?: Record<string, unknown>) => api.get('/audit-log', { params }),
}

export const importExportApi = {
  exportMonitors: () => api.get('/export/monitors'),
  importMonitors: (data: Record<string, unknown>) => api.post('/import/monitors', data),
}
