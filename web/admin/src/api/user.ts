import axios from 'axios';
export interface LoginData { username: string; password: string }
export function login(data: LoginData) { return axios.post<{access_token:string}>('/api/v1/auth/login',data); }
export function logout() { return axios.post('/api/v1/auth/logout'); }
export function getUserInfo() { return axios.get<{id:number;username:string}>('/api/v1/auth/me'); }
export async function getMenuList() { return {data: []}; }
