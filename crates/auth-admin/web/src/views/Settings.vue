<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import {
  getSettings, updatePasswordPolicy, updateCaptchaSettings, update2FASettings,
  type PasswordPolicySettings, type CaptchaSettings, type TwoFactorSettings
} from '../api'

const activeTab = ref('captcha')
const settingsLoading = ref(false)
const saving = ref(false)

// 验证码设置
const captchaForm = reactive<CaptchaSettings>({ enabled: false })

// 密码策略
const passwordForm = reactive<PasswordPolicySettings>({
  min_length: 6,
  require_uppercase: false,
  require_lowercase: false,
  require_digit: false,
  require_special: false
})

// 2FA 设置
const twoFaForm = reactive<TwoFactorSettings>({ enabled: false, methods: [], grace_days: 0 })
const twoFaMethodOptions = [
  { label: 'TOTP 验证器', value: 'totp' },
  { label: '短信验证码', value: 'sms' },
  { label: '二次密码', value: 'second_password' }
]

async function loadSettings() {
  settingsLoading.value = true
  try {
    const data = await getSettings()
    Object.assign(captchaForm, data.captcha)
    Object.assign(passwordForm, data.password)
    Object.assign(twoFaForm, data.two_fa)
  } catch (e: any) {
    ElMessage.error(e.message || '加载设置失败')
  } finally {
    settingsLoading.value = false
  }
}

async function saveCaptcha() {
  saving.value = true
  try {
    await updateCaptchaSettings({ enabled: captchaForm.enabled })
    ElMessage.success('验证码设置已保存')
  } catch (e: any) {
    ElMessage.error(e.message || '保存失败')
  } finally {
    saving.value = false
  }
}

async function savePassword() {
  saving.value = true
  try {
    await updatePasswordPolicy({ ...passwordForm })
    ElMessage.success('密码策略已保存')
  } catch (e: any) {
    ElMessage.error(e.message || '保存失败')
  } finally {
    saving.value = false
  }
}

async function save2FA() {
  saving.value = true
  try {
    await update2FASettings({ ...twoFaForm })
    ElMessage.success('2FA 设置已保存')
  } catch (e: any) {
    ElMessage.error(e.message || '保存失败')
  } finally {
    saving.value = false
  }
}

onMounted(() => { loadSettings() })
</script>

<template>
  <div class="settings-page">
    <div class="page-header">
      <h2 class="page-title">系统设置</h2>
      <p class="page-desc">配置认证安全策略，修改后立即生效</p>
    </div>

    <el-card v-loading="settingsLoading">
      <el-tabs v-model="activeTab">
        <!-- 验证码设置 -->
        <el-tab-pane label="验证码设置" name="captcha">
          <el-form label-width="120px" style="max-width: 500px">
            <el-form-item label="启用验证码">
              <el-switch v-model="captchaForm.enabled" />
              <span style="margin-left: 8px; color: #909399;">{{ captchaForm.enabled ? '已启用' : '已禁用' }}</span>
            </el-form-item>
            <el-form-item>
              <el-button type="primary" :loading="saving" @click="saveCaptcha">保存</el-button>
            </el-form-item>
          </el-form>
          <el-alert
            type="info"
            :closable="false"
            style="margin-top: 16px; max-width: 500px;"
          >
            启用后，登录页将显示图片验证码，用户需先正确输入验证码才能登录。
          </el-alert>
        </el-tab-pane>

        <!-- 密码策略 -->
        <el-tab-pane label="密码策略" name="password">
          <el-form label-width="140px" style="max-width: 500px">
            <el-form-item label="最小长度">
              <el-input-number v-model="passwordForm.min_length" :min="0" :max="64" />
              <span v-if="passwordForm.min_length === 0" style="margin-left: 8px; color: #e6a23c;">设为 0 表示不启用密码策略</span>
            </el-form-item>
            <el-divider content-position="left">复杂度要求</el-divider>
            <el-form-item label="要求大写字母">
              <el-switch v-model="passwordForm.require_uppercase" />
            </el-form-item>
            <el-form-item label="要求小写字母">
              <el-switch v-model="passwordForm.require_lowercase" />
            </el-form-item>
            <el-form-item label="要求数字">
              <el-switch v-model="passwordForm.require_digit" />
            </el-form-item>
            <el-form-item label="要求特殊字符">
              <el-switch v-model="passwordForm.require_special" />
            </el-form-item>
            <el-form-item>
              <el-button type="primary" :loading="saving" @click="savePassword">保存</el-button>
            </el-form-item>
          </el-form>
          <el-alert
            type="info"
            :closable="false"
            style="margin-top: 16px; max-width: 500px;"
          >
            当最小长度设为 0 时，密码策略不生效，所有密码均可通过。启用复杂度要求后，创建用户和修改密码时均需满足对应条件。
          </el-alert>
        </el-tab-pane>

        <!-- 双因素认证 -->
        <el-tab-pane label="双因素认证" name="2fa">
          <el-form label-width="120px" style="max-width: 500px">
            <el-form-item label="启用 2FA">
              <el-switch v-model="twoFaForm.enabled" />
              <span style="margin-left: 8px; color: #909399;">{{ twoFaForm.enabled ? '已启用' : '已禁用' }}</span>
            </el-form-item>
            <el-form-item label="允许的方式">
              <el-checkbox-group v-model="twoFaForm.methods">
                <el-checkbox v-for="opt in twoFaMethodOptions" :key="opt.value" :label="opt.value" :value="opt.value">
                  {{ opt.label }}
                </el-checkbox>
              </el-checkbox-group>
            </el-form-item>
            <el-form-item v-if="twoFaForm.enabled" label="宽限期（天）">
              <el-input-number v-model="twoFaForm.grace_days" :min="0" :max="365" />
              <span style="margin-left: 8px; color: #909399; font-size: 12px;">0 表示立即强制；设置 N 天后未绑定 2FA 的用户将被强制跳转设置页面</span>
            </el-form-item>
            <el-form-item>
              <el-button type="primary" :loading="saving" @click="save2FA">保存</el-button>
            </el-form-item>
          </el-form>
          <el-alert
            type="info"
            :closable="false"
            style="margin-top: 16px; max-width: 500px;"
          >
            启用后，已绑定 2FA 的用户登录时将需要二次验证。支持 TOTP 验证器（如 Google Authenticator）、短信验证码和二次密码三种方式。
          </el-alert>
        </el-tab-pane>
      </el-tabs>
    </el-card>
  </div>
</template>

<style scoped>
.settings-page { height: 100%; display: flex; flex-direction: column; }

.page-header { margin-bottom: 20px; }

.page-title {
  margin: 0;
  font-size: 18px;
  color: #303133;
}

.page-desc {
  margin: 4px 0 0;
  font-size: 13px;
  color: #909399;
}
</style>
