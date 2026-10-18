import axios from 'axios';
import { Message } from '@arco-design/web-vue';
import { getToken, clearToken } from '@/utils/auth';
axios.defaults.timeout = 30000;
axios.interceptors.request.use(config => {
 const token = getToken(); if (token) config.headers.Authorization = `Bearer ${token}`;
 return config;
});
axios.interceptors.response.use(response => response, error => {
 const message = error.response?.data?.message || '网络请求失败';
 if (error.response?.status === 401) { clearToken(); if (!location.pathname.endsWith('/login')) location.assign(`${import.meta.env.BASE_URL}login`); }
 Message.error(message); return Promise.reject(new Error(message));
});
