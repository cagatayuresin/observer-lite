import { api } from './client'

export const authApi = {
  login: (username: string, password: string) =>
    api.post('/auth/login', { username, password }),

  refresh: (refresh_token: string) =>
    api.post('/auth/refresh', { refresh_token }),

  me: () => api.get('/auth/me'),

  changePassword: (current_password: string, new_password: string) =>
    api.post('/auth/change-password', { current_password, new_password }),
}
