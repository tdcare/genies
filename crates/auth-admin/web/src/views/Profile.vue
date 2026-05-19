<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { getMe, changePassword, get2FAStatus, setupTOTP, confirmTOTP, setupSecondPassword, setupSMS, verifySMS, disable2FA } from '../api'

interface UserInfo {
  id?: number
  username: string
  display_name: string
  email?: string
  phone?: string
  avatar?: string
  status: number
}

const userInfo = ref<UserInfo | null>(null)
const loading = ref(false)

async function loadProfile() {
  loading.value = true
  try {
    userInfo.value = await getMe()
  } catch (e: any) {
    ElMessage.error(e.message || '获取信息失败')
  } finally {
    loading.value = false
  }
}

// 修改密码弹窗
const pwdVisible = ref(false)
const pwdForm = ref({ oldPassword: '', newPassword: '', confirmPassword: '' })

async function handleChangePassword() {
  if (!pwdForm.value.oldPassword || !pwdForm.value.newPassword) {
    ElMessage.warning('请填写完整信息')
    return
  }
  if (pwdForm.value.newPassword !== pwdForm.value.confirmPassword) {
    ElMessage.warning('两次密码不一致')
    return
  }
  try {
    await changePassword(pwdForm.value.oldPassword, pwdForm.value.newPassword)
    ElMessage.success('密码修改成功')
    pwdVisible.value = false
    pwdForm.value = { oldPassword: '', newPassword: '', confirmPassword: '' }
  } catch (e: any) {
    ElMessage.error(e.message || '修改失败')
  }
}

// 2FA 状态
const fa2Loading = ref(false)
const fa2Enabled = ref(false)
const fa2Method = ref('')
const allowedMethods = ref<string[]>([])

// 2FA 强制设置状态
const require2FaSetup = ref(false)
const twoFaSetupDeadline = ref(0)

const deadlineText = computed(() => {
  if (!twoFaSetupDeadline.value) return ''
  const d = new Date(twoFaSetupDeadline.value * 1000)
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
})

function clear2FaEnforcement() {
  localStorage.removeItem('require_2fa_setup')
  localStorage.removeItem('two_fa_setup_deadline')
  require2FaSetup.value = false
  twoFaSetupDeadline.value = 0
}

async function load2FAStatus() {
  fa2Loading.value = true
  try {
    const status = await get2FAStatus()
    fa2Enabled.value = status.enabled
    fa2Method.value = status.method
    allowedMethods.value = (status as any).allowed_methods || []
  } catch { /* ignore */ }
  finally { fa2Loading.value = false }
}

// TOTP 设置
const totpStep = ref<'idle' | 'setup' | 'verify'>('idle')
const totpSecret = ref('')
const totpQrSvg = ref('')
const totpCode = ref('')
const backupCodes = ref<string[]>([])

async function handleSetupTOTP() {
  try {
    const data = await setupTOTP()
    totpSecret.value = data.secret
    totpQrSvg.value = data.qr_svg
    totpStep.value = 'setup'
  } catch (e: any) {
    ElMessage.error(e.message || '发起 TOTP 绑定失败')
  }
}

async function handleConfirmTOTP() {
  if (!totpCode.value) {
    ElMessage.warning('请输入 TOTP 验证码')
    return
  }
  try {
    const data = await confirmTOTP(totpCode.value)
    backupCodes.value = data.backup_codes
    totpStep.value = 'verify'
    fa2Enabled.value = true
    fa2Method.value = 'totp'
    ElMessage.success('TOTP 绑定成功')
    clear2FaEnforcement()
  } catch (e: any) {
    ElMessage.error(e.message || '确认失败')
  }
}

function resetTotp() {
  totpStep.value = 'idle'
  totpSecret.value = ''
  totpQrSvg.value = ''
  totpCode.value = ''
  backupCodes.value = []
}

// 二次密码
const secondPwdVisible = ref(false)
const secondPwdForm = ref({ password: '', confirmPassword: '' })

async function handleSetupSecondPassword() {
  if (!secondPwdForm.value.password || secondPwdForm.value.password.length < 4) {
    ElMessage.warning('二次密码至少需要4位')
    return
  }
  if (secondPwdForm.value.password !== secondPwdForm.value.confirmPassword) {
    ElMessage.warning('两次密码不一致')
    return
  }
  try {
    await setupSecondPassword(secondPwdForm.value.password)
    ElMessage.success('二次密码设置成功')
    clear2FaEnforcement()
    secondPwdVisible.value = false
    secondPwdForm.value = { password: '', confirmPassword: '' }
    fa2Enabled.value = true
    fa2Method.value = 'second_password'
  } catch (e: any) {
    ElMessage.error(e.message || '设置失败')
  }
}

// 关闭 2FA
async function handleDisable2FA() {
  try {
    await ElMessageBox.confirm('确定要关闭双因素认证吗？', '确认', { type: 'warning' })
    await disable2FA()
    ElMessage.success('2FA 已关闭')
    fa2Enabled.value = false
    fa2Method.value = ''
    resetTotp()
    resetSms()
  } catch { /* 取消 */ }
}

// 短信 2FA 设置
const smsPhone = ref('')
const smsCode = ref('')
const smsSending = ref(false)
const smsSent = ref(false)
const smsVerifying = ref(false)

async function handleSetupSMS() {
  if (!smsPhone.value || smsPhone.value.length < 11) {
    ElMessage.warning('请输入有效的手机号码')
    return
  }
  smsSending.value = true
  try {
    await setupSMS(smsPhone.value)
    smsSent.value = true
    ElMessage.success('验证码已发送')
  } catch (e: any) {
    ElMessage.error(e.message || '发送失败')
  } finally {
    smsSending.value = false
  }
}

async function handleVerifySMS() {
  if (!smsCode.value) {
    ElMessage.warning('请输入验证码')
    return
  }
  smsVerifying.value = true
  try {
    await verifySMS(smsCode.value)
    ElMessage.success('短信 2FA 绑定成功')
    clear2FaEnforcement()
    fa2Enabled.value = true
    fa2Method.value = 'sms'
    resetSms()
  } catch (e: any) {
    ElMessage.error(e.message || '验证失败')
  } finally {
    smsVerifying.value = false
  }
}

function resetSms() {
  smsPhone.value = ''
  smsCode.value = ''
  smsSent.value = false
}

function methodName(m: string): string {
  const map: Record<string, string> = { totp: 'TOTP 验证器', sms: '短信验证码', second_password: '二次密码' }
  return map[m] || m
}

onMounted(() => {
  loadProfile()
  load2FAStatus()
  require2FaSetup.value = localStorage.getItem('require_2fa_setup') === 'true'
  twoFaSetupDeadline.value = Number(localStorage.getItem('two_fa_setup_deadline') || '0')
})
</script>

<template>
  <div class="profile-page">
    <div class="page-header">
      <h2 class="page-title">个人信息</h2>
    </div>

    <el-card v-loading="loading" class="profile-card">
      <template v-if="userInfo">
        <el-descriptions :column="2" border>
          <el-descriptions-item label="用户名">{{ userInfo.username }}</el-descriptions-item>
          <el-descriptions-item label="显示名称">{{ userInfo.display_name }}</el-descriptions-item>
          <el-descriptions-item label="邮箱">{{ userInfo.email || '-' }}</el-descriptions-item>
          <el-descriptions-item label="手机号">{{ userInfo.phone || '-' }}</el-descriptions-item>
          <el-descriptions-item label="状态">
            <el-tag :type="userInfo.status === 1 ? 'success' : 'danger'">
              {{ userInfo.status === 1 ? '启用' : '禁用' }}
            </el-tag>
          </el-descriptions-item>
        </el-descriptions>

        <div style="margin-top: 24px">
          <el-button type="primary" @click="pwdVisible = true">修改密码</el-button>
        </div>
      </template>
    </el-card>

    <!-- 2FA 设置 -->
    <el-card v-loading="fa2Loading" class="fa2-card">
      <template #header>
        <span>双因素认证</span>
      </template>

      <div v-if="fa2Enabled" class="fa2-status">
        <el-tag type="success">{{ methodName(fa2Method) }} — 已启用</el-tag>
        <el-button type="danger" style="margin-left: 12px" @click="handleDisable2FA">关闭 2FA</el-button>
      </div>

      <div v-else-if="allowedMethods.length > 0">
        <p style="color: #909399; margin: 0 0 12px;">尚未启用双因素认证，选择一种方式进行设置：</p>

        <!-- TOTP 设置流程 -->
        <div v-if="allowedMethods.includes('totp')" class="fa2-section">
          <el-divider content-position="left">TOTP 验证器</el-divider>

          <!-- 初始状态 -->
          <div v-if="totpStep === 'idle'">
            <el-button type="primary" @click="handleSetupTOTP">绑定 TOTP</el-button>
          </div>

          <!-- 显示二维码 -->
          <div v-if="totpStep === 'setup'" class="totp-setup">
            <p style="color: #606266;">使用 Google Authenticator 或类似应用扫描以下二维码：</p>
            <div class="qr-container" v-html="totpQrSvg"></div>
            <p style="color: #909399; font-size: 12px;">或手动输入密钥：<code>{{ totpSecret }}</code></p>
            <div style="margin-top: 12px;">
              <el-input
                v-model="totpCode"
                placeholder="输入 6 位验证码确认"
                maxlength="6"
                style="width: 200px;"
              />
              <el-button type="primary" style="margin-left: 8px;" @click="handleConfirmTOTP">确认</el-button>
              <el-button @click="resetTotp">取消</el-button>
            </div>
          </div>

          <!-- 显示备用码 -->
          <div v-if="totpStep === 'verify'" class="totp-done">
            <el-alert type="success" :closable="false" title="TOTP 绑定成功" />
            <div style="margin-top: 12px;">
              <p style="color: #606266; margin: 0 0 8px;">备用恢复码（请妥善保存）：</p>
              <div class="backup-codes">
                <code v-for="(code, i) in backupCodes" :key="i">{{ code }}</code>
              </div>
            </div>
            <el-button style="margin-top: 12px;" @click="resetTotp">关闭</el-button>
          </div>
        </div>

        <!-- 二次密码设置 -->
        <div v-if="allowedMethods.includes('second_password')" class="fa2-section">
          <el-divider content-position="left">二次密码</el-divider>
          <el-button @click="secondPwdVisible = true">设置二次密码</el-button>
        </div>

        <!-- 短信验证码设置 -->
        <div v-if="allowedMethods.includes('sms')" class="fa2-section">
          <el-divider content-position="left">短信验证码</el-divider>
          <div style="display: flex; align-items: center; gap: 8px; margin-bottom: 8px;">
            <el-input
              v-model="smsPhone"
              placeholder="请输入手机号码"
              maxlength="11"
              :disabled="smsSent"
              style="width: 200px;"
            />
            <el-button
              type="primary"
              :loading="smsSending"
              :disabled="smsSent"
              @click="handleSetupSMS"
            >
              {{ smsSent ? '已发送' : '发送验证码' }}
            </el-button>
          </div>
          <div v-if="smsSent" style="display: flex; align-items: center; gap: 8px;">
            <el-input
              v-model="smsCode"
              placeholder="输入 6 位验证码"
              maxlength="6"
              style="width: 200px;"
            />
            <el-button
              type="success"
              :loading="smsVerifying"
              @click="handleVerifySMS"
            >
              确认绑定
            </el-button>
            <el-button @click="resetSms">取消</el-button>
          </div>
        </div>
      </div>
      <div v-else>
        <p style="color: #909399;">系统未启用双因素认证</p>
      </div>
    </el-card>

    <!-- 修改密码弹窗 -->
    <el-dialog v-model="pwdVisible" title="修改密码" width="420px">
      <el-form :model="pwdForm" label-width="100px">
        <el-form-item label="旧密码" required>
          <el-input v-model="pwdForm.oldPassword" type="password" show-password />
        </el-form-item>
        <el-form-item label="新密码" required>
          <el-input v-model="pwdForm.newPassword" type="password" show-password />
        </el-form-item>
        <el-form-item label="确认密码" required>
          <el-input v-model="pwdForm.confirmPassword" type="password" show-password />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="pwdVisible = false">取消</el-button>
        <el-button type="primary" @click="handleChangePassword">确定</el-button>
      </template>
    </el-dialog>

    <!-- 二次密码弹窗 -->
    <el-dialog v-model="secondPwdVisible" title="设置二次密码" width="420px">
      <el-form :model="secondPwdForm" label-width="100px">
        <el-form-item label="二次密码" required>
          <el-input v-model="secondPwdForm.password" type="password" show-password placeholder="至少4位" />
        </el-form-item>
        <el-form-item label="确认密码" required>
          <el-input v-model="secondPwdForm.confirmPassword" type="password" show-password />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="secondPwdVisible = false">取消</el-button>
        <el-button type="primary" @click="handleSetupSecondPassword">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped>
.profile-page {
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow-y: auto;
}

.page-header { margin-bottom: 20px; }

.page-title {
  margin: 0;
  font-size: 18px;
  color: #303133;
}

.profile-card { max-width: 700px; }

.fa2-enforce-alert {
  max-width: 700px;
  margin-bottom: 16px;
}

.fa2-card {
  max-width: 700px;
  margin-top: 20px;
}

.fa2-status {
  display: flex;
  align-items: center;
}

.fa2-section {
  margin-top: 8px;
}

.totp-setup .qr-container {
  margin: 12px 0;
  padding: 12px;
  background: #f5f7fa;
  border-radius: 8px;
  display: inline-block;
}

.totp-setup code {
  color: #409eff;
  word-break: break-all;
}

.backup-codes {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.backup-codes code {
  background: #f5f7fa;
  padding: 4px 8px;
  border-radius: 4px;
  font-size: 13px;
}
</style>
