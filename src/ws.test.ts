import assert from 'node:assert/strict'
import test from 'node:test'

import { parseGraphUpdateMessage } from './ws.ts'

test('parses added note node from graph_update message', () => {
  const result = parseGraphUpdateMessage(
    JSON.stringify({
      type: 'graph_update',
      delta: {
        added_nodes: [
          { id: '1', node_type: 'note', x: 10, y: 20, content: 'hello' },
        ],
        updated_nodes: [],
      },
    })
  )

  assert.equal(result.addedNodes.length, 1)
  assert.deepEqual(result.addedNodes[0], {
    id: '1',
    type: 'note',
    x: 10,
    y: 20,
    content: 'hello',
  })
})

test('ignores unsupported pushed node types', () => {
  const result = parseGraphUpdateMessage(
    JSON.stringify({
      type: 'graph_update',
      delta: {
        added_nodes: [
          { id: '1', node_type: 'arrow', x: 0, y: 0, content: '' },
        ],
        updated_nodes: [],
      },
    })
  )

  assert.equal(result.addedNodes.length, 0)
})
