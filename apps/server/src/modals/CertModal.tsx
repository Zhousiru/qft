import {
  Alert,
  Box,
  Button,
  Flex,
  Icon,
  Input,
  Modal,
  ModalBody,
  ModalCloseButton,
  ModalContent,
  ModalFooter,
  ModalHeader,
  ModalOverlay,
} from '@chakra-ui/react'
import { invoke, shell } from '@tauri-apps/api'
import { appDataDir, resolve } from '@tauri-apps/api/path'
import { Dispatch, SetStateAction, useEffect, useState } from 'react'
import { MdLock } from 'react-icons/md'
import { toast } from '../common/toast'

export function CertModal({
  isOpen,
  onClose,
  needGenCert,
  setNeedGenCert,
}: {
  isOpen: boolean
  onClose: () => void
  needGenCert: boolean | null
  setNeedGenCert: Dispatch<SetStateAction<boolean | null>>
}) {
  const [certPath, setCertPath] = useState('')
  const [keyPath, setKeyPath] = useState('')

  useEffect(() => {
    if (needGenCert !== false) {
      return
    }
    ;(async () => {
      const appDataPath = await appDataDir()
      setCertPath(await resolve(appDataPath, 'cert', 'cert.der'))
      setKeyPath(await resolve(appDataPath, 'cert', 'key.der'))
    })()
  }, [needGenCert])

  async function handleGenCert() {
    await invoke('gen_cert')
    toast({
      title: '生成成功',
      status: 'success',
    })
    setNeedGenCert(false)
  }

  async function handleOpenDir() {
    await shell.open(await resolve(await appDataDir(), 'cert'))
  }

  return (
    <>
      <Modal isOpen={isOpen} onClose={onClose}>
        <ModalOverlay />
        <ModalContent>
          <ModalHeader>TLS 证书</ModalHeader>
          <ModalCloseButton />

          <ModalBody display="flex" flexDirection="column" gap={2}>
            {needGenCert ? (
              <p>未找到 TLS 证书，请在下方生成</p>
            ) : (
              <p>TLS 证书已生成，请将证书添加至客户端信任根中</p>
            )}

            <Alert status="success" textAlign="justify">
              QFT 基于 QUIC 协议实现，传输过程中使用 TLS
              对文件数据进行非对称加密，避免 MITM
              攻击以及数据泄露，确保数据安全无虞
            </Alert>

            <Flex justifyContent="end">
              <Button
                colorScheme="blue"
                variant="outline"
                size="sm"
                onClick={handleOpenDir}
              >
                打开文件夹
              </Button>
            </Flex>

            <Box position="relative">
              <Flex
                direction="column"
                gap={2}
                opacity={needGenCert ? 0.5 : 1}
                pointerEvents={needGenCert ? 'none' : 'auto'}
              >
                <div>
                  证书位置
                  <Input mt={1} type="text" value={certPath} isReadOnly />
                </div>
                <div>
                  证书私钥位置
                  <Input mt={1} type="text" value={keyPath} isReadOnly />
                </div>
              </Flex>
              {needGenCert && (
                <Flex
                  position="absolute"
                  inset={0}
                  alignItems="center"
                  justifyContent="center"
                >
                  <Button
                    leftIcon={<Icon as={MdLock} />}
                    colorScheme="blue"
                    position="absolute"
                    onClick={handleGenCert}
                  >
                    生成证书
                  </Button>
                </Flex>
              )}
            </Box>
          </ModalBody>

          <ModalFooter>
            <Button
              colorScheme={needGenCert ? 'gray' : 'blue'}
              onClick={onClose}
            >
              OK
            </Button>
          </ModalFooter>
        </ModalContent>
      </Modal>
    </>
  )
}
