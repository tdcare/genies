<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { login, verify2FA, getCaptcha, get2FAStatus, setupTOTP, confirmTOTP, setupSecondPassword, setupSMS, verifySMS, type LoginResponse } from '../api'
import { User, Lock, Refresh } from '@element-plus/icons-vue'

const router = useRouter()

// 第一屏：账号密码
const form = ref({ username: '', password: '' })
const loading = ref(false)

// 验证码
const captchaId = ref('')
const captchaImage = ref('')
const captchaText = ref('')
const showCaptcha = ref(false)

// 2FA
const step = ref<'login' | '2fa'>('login')
const preauthToken = ref('')
const twoFaMethods = ref<string[]>([])
const twoFaCode = ref('')
const twoFaMethod = ref('')

// 2FA 设置弹框
const fa2SetupVisible = ref(false)
const fa2Enabled = ref(false)
const fa2SetupMethod = ref('')
const allowedMethods = ref<string[]>([])
const fa2Loading = ref(false)

// TOTP
const totpStep = ref<'idle' | 'setup' | 'verify'>('idle')
const totpSecret = ref('')
const totpQrSvg = ref('')
const totpCode = ref('')
const backupCodes = ref<string[]>([])

// 二次密码弹窗
const secondPwdVisible = ref(false)
const secondPwdForm = ref({ password: '', confirmPassword: '' })

// 短信
const smsPhone = ref('')
const smsCode = ref('')
const smsSending = ref(false)
const smsSent = ref(false)
const smsVerifying = ref(false)

async function load2FAAllowedMethods() {
  fa2Loading.value = true
  try {
    const status = await get2FAStatus()
    fa2Enabled.value = status.enabled
    fa2SetupMethod.value = status.method
    allowedMethods.value = (status as any).allowed_methods || []
  } catch { /* ignore */ }
  finally { fa2Loading.value = false }
}

function clearEnforcement() {
  localStorage.removeItem('require_2fa_setup')
  localStorage.removeItem('two_fa_setup_deadline')
}

async function handleSetupTOTP_dialog() {
  try {
    const data = await setupTOTP()
    totpSecret.value = data.secret
    totpQrSvg.value = data.qr_svg
    totpStep.value = 'setup'
  } catch (e: any) {
    ElMessage.error(e.message || '发起 TOTP 绑定失败')
  }
}

async function handleConfirmTOTP_dialog() {
  if (!totpCode.value) { ElMessage.warning('请输入 TOTP 验证码'); return }
  try {
    const data = await confirmTOTP(totpCode.value)
    backupCodes.value = data.backup_codes
    totpStep.value = 'verify'
    fa2Enabled.value = true
    fa2SetupMethod.value = 'totp'
    ElMessage.success('TOTP 绑定成功')
    clearEnforcement()
  } catch (e: any) { ElMessage.error(e.message || '确认失败') }
}

function resetTotp() { totpStep.value = 'idle'; totpSecret.value = ''; totpQrSvg.value = ''; totpCode.value = ''; backupCodes.value = [] }

async function handleSetupSecondPassword_dialog() {
  if (!secondPwdForm.value.password || secondPwdForm.value.password.length < 4) { ElMessage.warning('二次密码至少需要4位'); return }
  if (secondPwdForm.value.password !== secondPwdForm.value.confirmPassword) { ElMessage.warning('两次密码不一致'); return }
  try {
    await setupSecondPassword(secondPwdForm.value.password)
    ElMessage.success('二次密码设置成功')
    clearEnforcement()
    secondPwdVisible.value = false
    secondPwdForm.value = { password: '', confirmPassword: '' }
    fa2Enabled.value = true
    fa2SetupMethod.value = 'second_password'
  } catch (e: any) { ElMessage.error(e.message || '设置失败') }
}

async function handleSetupSMS_dialog() {
  if (!smsPhone.value || smsPhone.value.length < 11) { ElMessage.warning('请输入有效的手机号码'); return }
  smsSending.value = true
  try { await setupSMS(smsPhone.value); smsSent.value = true; ElMessage.success('验证码已发送') }
  catch (e: any) { ElMessage.error(e.message || '发送失败') }
  finally { smsSending.value = false }
}

async function handleVerifySMS_dialog() {
  if (!smsCode.value) { ElMessage.warning('请输入验证码'); return }
  smsVerifying.value = true
  try {
    await verifySMS(smsCode.value)
    ElMessage.success('短信 2FA 绑定成功')
    clearEnforcement()
    fa2Enabled.value = true
    fa2SetupMethod.value = 'sms'
    resetSms()
  } catch (e: any) { ElMessage.error(e.message || '验证失败') }
  finally { smsVerifying.value = false }
}

function resetSms() { smsPhone.value = ''; smsCode.value = ''; smsSent.value = false }

function open2FASetupDialog() {
  totpStep.value = 'idle'
  resetTotp()
  resetSms()
  secondPwdForm.value = { password: '', confirmPassword: '' }
  load2FAAllowedMethods()
  fa2SetupVisible.value = true
}

function goToHome() { fa2SetupVisible.value = false; router.push('/users') }

async function loadCaptcha() {
  try {
    const data = await getCaptcha()
    captchaId.value = data.captcha_id
    captchaImage.value = data.image_base64
    showCaptcha.value = true
  } catch {
    showCaptcha.value = false
  }
}

async function refreshCaptcha() {
  await loadCaptcha()
  captchaText.value = ''
}

// 2FA 强制设置弹框：在登录成功后、跳转前调用
async function showEnforcementDialog(deadlineNum: number, isForced: boolean) {
  const deadlineText = deadlineNum > 0
    ? (() => { const d = new Date(deadlineNum * 1000); return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}` })()
    : ''

  if (isForced) {
    // 强制设置：单按钮，不可关闭
    await ElMessageBox.alert(
      deadlineText
        ? `系统要求您启用双因素认证，请在 ${deadlineText} 前完成设置，否则将无法使用系统其他功能。`
        : '系统要求您启用双因素认证，请立即完成设置，否则将无法使用系统其他功能。',
      '双因素认证强制设置',
      {
        confirmButtonText: '马上设置',
        type: 'error',
        showClose: false,
        closeOnClickModal: false,
        closeOnPressEscape: false,
      }
    )
    open2FASetupDialog()
  } else {
    // 宽限期：双按钮
    try {
      await ElMessageBox.confirm(
        `系统已启用双因素认证，请在 ${deadlineText} 前完成设置。宽限期内您仍可正常使用系统，请尽快完成 2FA 设置以避免到期后被限制访问。`,
        '双因素认证提醒',
        {
          confirmButtonText: '马上设置',
          cancelButtonText: '下次设置',
          type: 'warning',
        }
      )
      open2FASetupDialog()
    } catch {
      // 用户点击"下次设置"
      router.push('/users')
    }
  }
}

// 提交登录
async function handleLogin() {
  if (!form.value.username || !form.value.password) {
    ElMessage.warning('请输入用户名和密码')
    return
  }
  loading.value = true
  try {
    const resp = await login(
      form.value.username,
      form.value.password,
      showCaptcha.value ? captchaId.value : undefined,
      showCaptcha.value ? captchaText.value : undefined
    )

    if (resp.require_2fa) {
      // 进入二次验证
      step.value = '2fa'
      preauthToken.value = resp.preauth_token || ''
      twoFaMethods.value = resp.available_methods || []
      twoFaMethod.value = twoFaMethods.value[0] || ''
      twoFaCode.value = ''
    } else if (resp.require_2fa_setup || resp.two_fa_setup_deadline) {
      // 需要设置 2FA（强制或宽限期），弹框提示
      await showEnforcementDialog(resp.two_fa_setup_deadline || 0, !!resp.require_2fa_setup)
    } else {
      ElMessage.success('登录成功')
      router.push('/users')
    }
  } catch (e: any) {
    ElMessage.error(e.message || '登录失败')
    // 验证码错误时刷新
    if (showCaptcha.value) {
      refreshCaptcha()
    }
  } finally {
    loading.value = false
  }
}

// 提交 2FA 验证
async function handleVerify2FA() {
  if (!twoFaCode.value) {
    ElMessage.warning('请输入验证码')
    return
  }
  loading.value = true
  try {
    await verify2FA(preauthToken.value, twoFaCode.value, twoFaMethod.value)
    ElMessage.success('登录成功')
    router.push('/users')
  } catch (e: any) {
    ElMessage.error(e.message || '验证失败')
  } finally {
    loading.value = false
  }
}

// 返回登录页
function backToLogin() {
  step.value = 'login'
  preauthToken.value = ''
  twoFaCode.value = ''
}

// 获取 2FA 方式中文名
function methodLabel(m: string): string {
  const map: Record<string, string> = { totp: 'TOTP 验证器', sms: '短信验证码', second_password: '二次密码' }
  return map[m] || m
}

onMounted(() => { loadCaptcha() })
</script>

<template>
  <div class="login-container">
    <!-- 第一屏：账号密码 + 验证码 -->
    <div v-if="step === 'login'" class="login-box">
      <h2 class="login-title">统一认证管理</h2>
      <p class="login-subtitle">用户权限与组织架构管理</p>

      <el-form :model="form" @keyup.enter="handleLogin">
        <el-form-item>
          <el-input
            v-model="form.username"
            placeholder="用户名"
            :prefix-icon="User"
            size="large"
          />
        </el-form-item>
        <el-form-item>
          <el-input
            v-model="form.password"
            type="password"
            placeholder="密码"
            :prefix-icon="Lock"
            size="large"
            show-password
          />
        </el-form-item>

        <!-- 验证码区域 -->
        <el-form-item v-if="showCaptcha">
          <div class="captcha-row">
            <el-input
              v-model="captchaText"
              placeholder="验证码"
              size="large"
              maxlength="4"
              style="width: 60%"
            />
            <div class="captcha-image" @click="refreshCaptcha" title="点击刷新">
              <img
                v-if="captchaImage"
                :src="'data:image/png;base64,' + captchaImage"
                alt="验证码"
              />
              <el-icon class="captcha-refresh"><Refresh /></el-icon>
            </div>
          </div>
        </el-form-item>

        <el-form-item>
          <el-button
            type="primary"
            size="large"
            :loading="loading"
            style="width: 100%"
            @click="handleLogin"
          >
            登录
          </el-button>
        </el-form-item>
      </el-form>
    </div>

    <!-- 第二屏：2FA 验证 -->
    <div v-else class="login-box">
      <h2 class="login-title">二次验证</h2>
      <p class="login-subtitle">请输入验证码完成登录</p>

      <el-form @keyup.enter="handleVerify2FA">
        <!-- 方式选择 -->
        <el-form-item v-if="twoFaMethods.length > 1">
          <el-radio-group v-model="twoFaMethod">
            <el-radio-button v-for="m in twoFaMethods" :key="m" :value="m">
              {{ methodLabel(m) }}
            </el-radio-button>
          </el-radio-group>
        </el-form-item>

        <el-form-item>
          <el-input
            v-model="twoFaCode"
            :placeholder="'请输入' + methodLabel(twoFaMethod) + '验证码'"
            size="large"
            maxlength="8"
            clearable
          />
        </el-form-item>

        <el-form-item>
          <el-button
            type="primary"
            size="large"
            :loading="loading"
            style="width: 100%"
            @click="handleVerify2FA"
          >
            验证
          </el-button>
        </el-form-item>

        <el-form-item>
          <el-button style="width: 100%" @click="backToLogin">返回登录</el-button>
        </el-form-item>
      </el-form>
    </div>
  </div>

  <!-- 2FA 设置弹框 -->
  <el-dialog
    v-model="fa2SetupVisible"
    title="设置双因素认证"
    width="520px"
    :close-on-click-modal="false"
    :close-on-press-escape="false"
    :show-close="false"
    center
  >
    <!-- 已启用 -->
    <div v-if="fa2Enabled" style="text-align: center; padding: 20px 0;">
      <el-result icon="success" title="设置完成" :sub-title="'已启用 ' + methodLabel(fa2SetupMethod)">
        <template #extra>
          <el-button type="primary" @click="goToHome">进入系统</el-button>
        </template>
      </el-result>
    </div>

    <!-- 设置选项 -->
    <div v-else v-loading="fa2Loading">
      <p style="color: #909399; margin: 0 0 16px;">请选择一种验证方式进行设置：</p>

      <!-- TOTP -->
      <div v-if="allowedMethods.includes('totp')" class="fa2-block">
        <el-divider content-position="left">TOTP 验证器</el-divider>
        <div v-if="totpStep === 'idle'">
          <el-button type="primary" @click="handleSetupTOTP_dialog">绑定 TOTP</el-button>
        </div>
        <div v-if="totpStep === 'setup'" class="totp-setup">
          <p style="color: #606266;">扫描以下二维码：</p>
          <div class="qr-container" v-html="totpQrSvg"></div>
          <p style="color: #909399; font-size: 12px;">或手动输入密钥：<code>{{ totpSecret }}</code></p>
          <div style="margin-top: 12px;">
            <el-input v-model="totpCode" placeholder="输入 6 位验证码确认" maxlength="6" style="width: 200px;" />
            <el-button type="primary" style="margin-left: 8px;" @click="handleConfirmTOTP_dialog">确认</el-button>
            <el-button @click="resetTotp">取消</el-button>
          </div>
        </div>
        <div v-if="totpStep === 'verify'">
          <el-alert type="success" :closable="false" title="TOTP 绑定成功" />
          <div style="margin-top: 12px;">
            <p style="color: #606266; margin: 0 0 8px;">备用恢复码（请妥善保存）：</p>
            <div class="backup-codes">
              <code v-for="(c, i) in backupCodes" :key="i">{{ c }}</code>
            </div>
          </div>
          <el-button type="primary" style="margin-top: 12px;" @click="goToHome">进入系统</el-button>
        </div>
      </div>

      <!-- 二次密码 -->
      <div v-if="allowedMethods.includes('second_password')" class="fa2-block">
        <el-divider content-position="left">二次密码</el-divider>
        <el-button @click="secondPwdVisible = true">设置二次密码</el-button>
      </div>

      <!-- 短信验证码 -->
      <div v-if="allowedMethods.includes('sms')" class="fa2-block">
        <el-divider content-position="left">短信验证码</el-divider>
        <div style="display: flex; align-items: center; gap: 8px; margin-bottom: 8px;">
          <el-input v-model="smsPhone" placeholder="请输入手机号码" maxlength="11" :disabled="smsSent" style="width: 200px;" />
          <el-button type="primary" :loading="smsSending" :disabled="smsSent" @click="handleSetupSMS_dialog">{{ smsSent ? '已发送' : '发送验证码' }}</el-button>
        </div>
        <div v-if="smsSent" style="display: flex; align-items: center; gap: 8px;">
          <el-input v-model="smsCode" placeholder="输入 6 位验证码" maxlength="6" style="width: 200px;" />
          <el-button type="success" :loading="smsVerifying" @click="handleVerifySMS_dialog">确认绑定</el-button>
          <el-button @click="resetSms">取消</el-button>
        </div>
      </div>
    </div>
  </el-dialog>

  <!-- 二次密码子弹窗 -->
  <el-dialog v-model="secondPwdVisible" title="设置二次密码" width="400px" append-to-body>
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
      <el-button type="primary" @click="handleSetupSecondPassword_dialog">确定</el-button>
    </template>
  </el-dialog>
</template>

<style scoped>
.login-container {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
}

.login-box {
  width: 400px;
  padding: 40px;
  background: white;
  border-radius: 12px;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
}

.login-title {
  margin: 0 0 8px;
  text-align: center;
  font-size: 24px;
  color: #303133;
}

.login-subtitle {
  margin: 0 0 32px;
  text-align: center;
  font-size: 14px;
  color: #909399;
}

.captcha-row {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
}

.captcha-image {
  cursor: pointer;
  position: relative;
  flex-shrink: 0;
  width: 120px;
  height: 40px;
  border: 1px solid #dcdfe6;
  border-radius: 4px;
  overflow: hidden;
}

.captcha-image img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.captcha-refresh {
  position: absolute;
  top: 2px;
  right: 2px;
  font-size: 12px;
  color: #909399;
  background: rgba(255,255,255,0.8);
  border-radius: 2px;
}

/* ---- 2FA 设置弹框样式 ---- */
.fa2-block {
  margin-top: 8px;
}

.totp-setup .qr-container {
  margin: 8px 0;
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
