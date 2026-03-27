import type { NotificationItem } from '../types'
import { escapeHtml } from './common'

export const renderNotifications = (notifications: NotificationItem[]): string => {
  if (notifications.length === 0) {
    return '<div class="notifications"></div>'
  }
  const items = notifications
    .map(
      (notification) =>
        `<div class="notice notice-${notification.level}"><span>${escapeHtml(
          notification.message
        )}</span><button data-action="dismiss-notice" data-id="${notification.id}">Dismiss</button></div>`
    )
    .join('')
  return `<div class="notifications">${items}</div>`
}
