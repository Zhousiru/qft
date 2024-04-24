import {
  Badge,
  Box,
  Button,
  Card,
  Flex,
  Icon,
  Text,
  useDisclosure,
} from '@chakra-ui/react'
import { BaseDirectory, exists } from '@tauri-apps/api/fs'
import { join } from '@tauri-apps/api/path'
import { useEffect, useState } from 'react'
import { MdRocketLaunch } from 'react-icons/md'
import { CertModal } from './modals/CertModal'

function App() {
  const certModal = useDisclosure()
  const [needGenCert, setNeedGenCert] = useState<boolean | null>(null)

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
              isDisabled={needGenCert}
            >
              启动服务端
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

      <Flex flexGrow={1} direction="column" p={2}>
        <Card variant="outline">
          <Flex
            gap={2}
            px={4}
            py={2}
            alignItems="center"
            borderBottomWidth={1}
            borderBottomColor="gray.200"
          >
            <Text fontSize={18}>测试文件名.zip</Text>
            <Badge colorScheme="blue" variant="outline">
              11.4 GB
            </Badge>
          </Flex>
          <Flex px={4} py={2} direction="column" gap={1}>
            <div>
              <Box w={100} textAlign="right">
                块总数：
              </Box>
            </div>
            <div>
              <Box w={100} textAlign="right">
                已重建块数：
              </Box>
            </div>
            <div>
              <Box w={100} textAlign="right">
                重传次数：
              </Box>
            </div>
            <div>
              <Box w={100} textAlign="right">
                总进度：
              </Box>
            </div>
            <div>
              <Box w={100} textAlign="right">
                状态：
              </Box>
            </div>
          </Flex>
        </Card>
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
