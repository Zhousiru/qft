import {
  Badge,
  Box,
  Button,
  Card,
  Flex,
  Icon,
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
import {
  MdCheckCircleOutline,
  MdList,
  MdOpenInNew,
  MdRocketLaunch,
} from 'react-icons/md'
import { toast } from './common/toast'
import { CertModal } from './modals/CertModal'
import { Task } from './types/task'

function App() {
  const certModal = useDisclosure()
  const [needGenCert, setNeedGenCert] = useState<boolean | null>(null)
  const [isStartup, setIsStartup] = useState(false)
  const [tasks, setTasks] = useState<Task[]>([])

  useEffect(() => {
    ;(async () => {
      if (
        !(await exists(await join('cert', 'cert.der'), {
          dir: BaseDirectory.AppData,
        })) ||
        !(await exists(await join('cert', 'key.der'), {
          dir: BaseDirectory.AppData,
        }))
      ) {
        setNeedGenCert(true)
      } else {
        setNeedGenCert(false)
      }
    })()
  }, [])

  useEffect(() => {
    const unlisenPromise = listen('task', (e) => {
      setIsStartup(true)
      const payload = e.payload as Task
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

  async function handleStart() {
    await invoke('start_server')
    setIsStartup(true)
    toast({
      title: '服务端已启动',
      status: 'success',
    })
  }

  async function handleOpenRecvFolder() {
    if (
      !(await exists('recv', {
        dir: BaseDirectory.AppData,
      }))
    ) {
      await createDir('recv', {
        dir: BaseDirectory.AppData,
        recursive: true,
      })
    }
    await shell.open(await resolve(await appDataDir(), 'recv'))
  }

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
        gap={2}
      >
        {needGenCert !== null && (
          <>
            <Button
              leftIcon={<Icon as={MdRocketLaunch} />}
              variant={!needGenCert ? 'solid' : 'outline'}
              colorScheme="blue"
              isDisabled={needGenCert || isStartup}
              onClick={handleStart}
            >
              {isStartup ? '服务端已启动' : '启动服务端'}
            </Button>
            <Button
              onClick={certModal.onOpen}
              variant={needGenCert ? 'solid' : 'outline'}
              colorScheme={needGenCert ? 'blue' : 'gray'}
              size="sm"
              position="absolute"
              bottom={4}
            >
              {needGenCert ? '生成 TLS 证书' : '查看 TLS 证书'}
            </Button>
          </>
        )}
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
            接收任务列表
          </Text>
          <Button
            ml="auto"
            variant="outline"
            size="xs"
            onClick={handleOpenRecvFolder}
          >
            <Icon as={MdOpenInNew} />
          </Button>
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
                  暂无接收任务
                </Box>
              )}

              {tasks.map((task) => (
                <Card variant="outline">
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
                      {task.status === 'recv' && '接收中'}
                      {task.status === 'merge' && '合并中'}
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
                        已重建块数：
                      </Box>
                      {task.doneBlockCount}
                    </Flex>
                    <Flex>
                      <Box w="90px" textAlign="right" textColor="GrayText">
                        重建进度：
                      </Box>
                      {((task.doneBlockCount / task.blockCount) * 100).toFixed(
                        1,
                      )}
                      %
                    </Flex>

                    <Progress
                      my={2}
                      size="sm"
                      value={(task.doneBlockCount / task.blockCount) * 100}
                    />
                  </Flex>
                </Card>
              ))}
            </Flex>
          </Box>
        </Flex>
      </Flex>

      <CertModal
        isOpen={certModal.isOpen}
        onClose={certModal.onClose}
        needGenCert={needGenCert}
        setNeedGenCert={setNeedGenCert}
      />
    </Flex>
  )
}

export default App
