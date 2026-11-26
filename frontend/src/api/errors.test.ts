import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { formatApiError } from './errors'
import { useToastStore } from '../stores/toast'

describe('formatApiError', () => {
  it('reads a string detail', () => {
    expect(formatApiError({ response: { status: 401, data: { detail: 'Invalid credentials' } } })).toBe(
      'Invalid credentials',
    )
  })

  it('joins validation messages', () => {
    expect(
      formatApiError({ response: { status: 422, data: { detail: [{ msg: 'field required' }, 'name'] } } }),
    ).toBe('field required name')
  })

  it('reports a network failure', () => {
    expect(formatApiError(new Error('offline'))).toBe('The server could not be reached.')
  })

  it('falls back to the status code', () => {
    expect(formatApiError({ response: { status: 500, data: {} } })).toBe('Request failed (500).')
  })
})

describe('toast store', () => {
  beforeEach(() => setActivePinia(createPinia()))

  it('stacks typed notices and removes one', () => {
    const toast = useToastStore()
    const id = toast.error('Disk full', 'Storage')
    expect(toast.toasts[0]).toMatchObject({
      type: 'error',
      title: 'Storage',
      message: 'Disk full',
      timeout: 9000,
    })
    toast.warning('Certificate expires soon')
    toast.success('Saved')
    toast.info('Check finished')
    expect(toast.toasts.map((item) => item.type)).toEqual(['error', 'warning', 'success', 'info'])
    toast.remove(id)
    expect(toast.toasts).toHaveLength(3)
  })
})
