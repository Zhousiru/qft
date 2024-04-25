export interface Task {
  filename: string
  fileSize: number
  pps: number
  uuid: string
  blockCount: number
  remainBlockCount: number
  status: 'send' | 'done'
}
