import { tickets } from '../.agentctl/lib/helpers.js'

export const meta = {
  id: 'default',
  name: 'Default Workflow',
  description: 'Process the next open ticket',
  runtime: 'host',
}

function assertOk(result, action) {
  if (result?.ok) return result
  throw new Error(`${action} failed: ${result?.error ?? 'unknown error'}`)
}

export default async function defaultWorkflow(ctx) {
  const next = await tickets.getNext({ status: 'OPEN' })
  if (!next.ok) {
    throw new Error(`Failed to fetch tickets: ${next.error}`)
  }
  if (!next.ticket) {
    ctx.log.info('No open tickets found')
    return { ok: true, skipped: true }
  }

  const ticket = next.ticket
  ctx.log.info(`Processing ticket ${ticket.ticketId}: ${ticket.title}`)

  assertOk(
    await tickets.updateStatus({
      id: ticket.ticketId,
      status: 'IN_PROGRESS',
    }),
    `Update ticket ${ticket.ticketId} to IN_PROGRESS`
  )

  const result = await ctx.agent.run({
    title: `Work on: ${ticket.title}`,
    prompt: `You have been assigned this ticket:\n\nTitle: ${ticket.title}\n${ticket.description ?? ''}\n\nImplement what is described. When done, summarize what you did.`,
  })

  if (!result.ok) {
    assertOk(
      await tickets.comment({
        id: ticket.ticketId,
        text: `Agent failed: ${result.error}`,
      }),
      `Comment on failed ticket ${ticket.ticketId}`
    )
    throw new Error(
      `Agent failed on ticket ${ticket.ticketId}: ${result.error}`
    )
  }

  assertOk(
    await tickets.comment({
      id: ticket.ticketId,
      text: result.output || 'Work completed.',
    }),
    `Comment on ticket ${ticket.ticketId}`
  )
  assertOk(
    await tickets.updateStatus({
      id: ticket.ticketId,
      status: 'QA_REVIEW',
    }),
    `Update ticket ${ticket.ticketId} to QA_REVIEW`
  )

  ctx.log.info(`Ticket ${ticket.ticketId} moved to QA_REVIEW`)
  return { ok: true, ticketId: ticket.ticketId }
}
