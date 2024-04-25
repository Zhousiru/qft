import {
  Alert,
  Button,
  Flex,
  Input,
  Modal,
  ModalBody,
  ModalCloseButton,
  ModalContent,
  ModalFooter,
  ModalHeader,
  ModalOverlay,
  Slider,
  SliderFilledTrack,
  SliderThumb,
  SliderTrack,
  Text,
} from '@chakra-ui/react'
import { invoke } from '@tauri-apps/api'
import { open } from '@tauri-apps/api/dialog'
import { filesize } from 'filesize'
import { useState } from 'react'
import { toast } from '../common/toast'

export function NewTaskModal({
  isOpen,
  onClose,
}: {
  isOpen: boolean
  onClose: () => void
}) {
  const [filePath, setFilePath] = useState('')
  const [pps, setPps] = useState(20000)

  async function handleCreateTask() {
    await invoke('send_file', { path: filePath, pps })
    toast({
      title: '传输任务已创建',
      status: 'success',
    })
    onClose()
  }

  async function handleSelectFile() {
    const selection = await open({ title: '选择将要传输的文件' })
    if (typeof selection !== 'string') {
      return
    }
    setFilePath(selection)
  }

  return (
    <>
      <Modal isOpen={isOpen} onClose={onClose}>
        <ModalOverlay />
        <ModalContent>
          <ModalHeader>创建传输任务</ModalHeader>
          <ModalCloseButton />

          <ModalBody display="flex" flexDirection="column" gap={2}>
            <Flex direction="column" gap={2}>
              <div>
                文件路径
                <Flex mt={1} gap={1}>
                  <Input
                    type="text"
                    value={filePath}
                    onChange={(e) => setFilePath(e.target.value)}
                  />
                  <Button px={4} variant="outline" onClick={handleSelectFile}>
                    选择
                  </Button>
                </Flex>
              </div>
              <div>
                包速率限制（PPS）
                <Slider
                  mt={1}
                  min={10000}
                  max={200000}
                  step={10000}
                  value={pps}
                  onChange={(v) => setPps(v)}
                >
                  <SliderTrack>
                    <SliderFilledTrack />
                  </SliderTrack>
                  <SliderThumb />
                </Slider>
                <Alert
                  status="info"
                  mt={1}
                  display="flex"
                  alignItems="start"
                  gap={1}
                  flexDirection="column"
                >
                  <Text>
                    QFT
                    使用恒定流量发包机制，包速率限制将会直接决定最大传输速率。
                  </Text>
                  <Text>当前限制为：{pps} 包 / 秒</Text>
                  <Text>
                    理论最大速率：{filesize(pps * 1024, { standard: 'jedec' })}
                    /s
                  </Text>
                </Alert>
              </div>
            </Flex>
          </ModalBody>

          <ModalFooter>
            <Button colorScheme="blue" onClick={handleCreateTask}>
              创建
            </Button>
          </ModalFooter>
        </ModalContent>
      </Modal>
    </>
  )
}
