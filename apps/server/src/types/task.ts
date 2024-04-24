export interface Task {
  filename: string
  fileSize: number
  uuid: string
  blockCount: number
  doneBlockCount: number
  status: 'recv' | 'merge' | 'done'
}
