import {
  Badge,
  Box,
  Button,
  Card,
  Flex,
  Icon,
  Input,
  Progress,
  Spinner,
  Text,
  useDisclosure,
} from '@chakra-ui/react'
import { invoke, shell } from '@tauri-apps/api'
import { listen } from '@tauri-apps/api/event'
import { BaseDirectory, createDir, exists } from '@tauri-apps/api/fs'
import { appDataDir, join, resolve } from '@tauri-apps/api/path'
import { filesize } from 'filesize'
import { useEffect, useState } from 'react'
import { MdCheckCircleOutline, MdLink, MdList, MdSend } from 'react-icons/md'
import { toast } from './common/toast'
import { NewTaskModal } from './modals/NewTaskModal'
import { Task } from './types/task'

function App() {
  const [serverAddr, setServerAddr] = useState('127.0.0.1:23333')
  const [connected, setConnected] = useState(false)
  const [tasks, setTasks] = useState<Task[]>([])

  const newTaskModal = useDisclosure()

  async function handleConnect() {
    if (
      !(await exists(await join('cert', 'cert.der'), {
        dir: BaseDirectory.AppData,
      }))
    ) {
      toast({ title: '请先配置信任证书', status: 'error' })
      return
    }

    await invoke('connect_to_server', { addr: serverAddr })
    setConnected(true)
    toast({ title: '已连接到服务端', status: 'success' })
  }

  async function handleOpenCertDir() {
    if (
      !(await exists('cert', {
        dir: BaseDirectory.AppData,
      }))
    ) {
      await createDir('cert', {
        dir: BaseDirectory.AppData,
        recursive: true,
      })
    }
    await shell.open(await resolve(await appDataDir(), 'cert'))
  }

  useEffect(() => {
    const unlisenPromise = listen('task', (e) => {
      setConnected(true)
      const payload = e.payload as Task
      console.log(payload)
      const currentIndex = tasks.findIndex((x) => x.uuid === payload.uuid)
      if (currentIndex === -1) {
        setTasks((prev) => [...prev, payload])
      } else {
        setTasks((prev) => [...prev.toSpliced(currentIndex, 1, payload)])
      }
    })

    return () => {
      unlisenPromise.then((unlisen) => unlisen())
    }
  }, [tasks])

  return (
    <Flex h="100vh">
      <Flex
        w={250}
        bg="gray.50"
        borderRightWidth="thin"
        borderRightColor="gray.200"
        direction="column"
        alignItems="center"
        justifyContent="center"
        gap={4}
      >
        {!connected ? (
          <>
            <Input
              type="text"
              variant="flushed"
              w="200px"
              textAlign="center"
              fontSize={24}
              value={serverAddr}
              onChange={(e) => setServerAddr(e.target.value)}
            />
            <Button
              colorScheme="blue"
              leftIcon={<Icon as={MdLink} />}
              onClick={handleConnect}
            >
              连接服务端
            </Button>
          </>
        ) : (
          <>
            <Button
              colorScheme="blue"
              leftIcon={<Icon as={MdSend} />}
              onClick={newTaskModal.onOpen}
            >
              创建传输任务
            </Button>
          </>
        )}

        <Button
          variant="outline"
          colorScheme="gray"
          size="sm"
          position="absolute"
          bottom={4}
          onClick={handleOpenCertDir}
        >
          配置信任证书
        </Button>
      </Flex>

      <Flex flexGrow={1} direction="column">
        <Flex
          py={2}
          px={4}
          borderBottomWidth="thin"
          borderBottomColor="gray.200"
          alignItems="center"
          gap={2}
        >
          <Icon as={MdList} boxSize={5} />
          <Text fontSize={14} fontWeight="bold">
            发送任务列表
          </Text>
        </Flex>
        <Flex flexGrow={1} direction="column" position="relative">
          <Box position="absolute" inset={0} overflowY="auto">
            <Flex direction="column" gap={2} p={2}>
              {tasks.length === 0 && (
                <Box
                  textAlign="center"
                  textColor="GrayText"
                  mt={4}
                  fontSize={14}
                >
                  暂无发送任务
                </Box>
              )}

              {tasks.map((task) => (
                <Card variant="outline" key={task.uuid}>
                  <Flex
                    gap={2}
                    px={4}
                    py={2}
                    alignItems="center"
                    borderBottomWidth="thin"
                    borderBottomColor="gray.200"
                  >
                    <Flex
                      w={4}
                      h={4}
                      alignItems="center"
                      justifyContent="center"
                    >
                      {task.status === 'done' ? (
                        <Icon
                          as={MdCheckCircleOutline}
                          boxSize="20px"
                          textColor="green.500"
                        />
                      ) : (
                        <Spinner size="sm" color="blue.500" />
                      )}
                    </Flex>
                    <Text fontSize={18}>{task.filename}</Text>
                    <Badge colorScheme="blue" variant="outline">
                      {filesize(task.fileSize, { standard: 'jedec' })}
                    </Badge>
                  </Flex>
                  <Flex px={4} py={2} direction="column" gap={1} fontSize={14}>
                    <Flex>
                      <Box w="90px" textAlign="right" textColor="GrayText">
                        状态：
                      </Box>
                      {task.status === 'send' && '发送中'}
                      {task.status === 'done' && '已完成'}
                    </Flex>
                    <Flex>
                      <Box w="90px" textAlign="right" textColor="GrayText">
                        块总数：
                      </Box>
                      {task.blockCount}
                    </Flex>
                    <Flex>
                      <Box w="90px" textAlign="right" textColor="GrayText">
                        已确认块数：
                      </Box>
                      {task.blockCount - task.remainBlockCount}
                    </Flex>
                    <Flex>
                      <Box w="90px" textAlign="right" textColor="GrayText">
                        确认进度：
                      </Box>
                      {(
                        ((task.blockCount - task.remainBlockCount) /
                          task.blockCount) *
                        100
                      ).toFixed(1)}
                      %
                    </Flex>

                    <Progress
                      my={2}
                      size="sm"
                      value={
                        ((task.blockCount - task.remainBlockCount) /
                          task.blockCount) *
                        100
                      }
                    />
                  </Flex>
                </Card>
              ))}
            </Flex>
          </Box>
        </Flex>
      </Flex>

      <NewTaskModal
        isOpen={newTaskModal.isOpen}
        onClose={newTaskModal.onClose}
      />
    </Flex>
  )
}

export default App
