import axios from 'axios';

export async function postRequest(
    url: string,
    data: Record<string, any>,
    headers?: Record<string, string>, // 修改为普通对象类型
    timeout: number = 5000 // 默认超时时间为 5000 毫秒
): Promise<{ statusCode: number; result: any }> {
    try {
        // 如果未指定 Content-Type，则默认设置为 application/json
        const defaultHeaders = { 'Content-Type': 'application/json', ...(headers || {}) };
        const response = await axios.post(url, data, { headers: defaultHeaders, timeout });
        return { statusCode: response.status, result: response.data };
    } catch (error: any) {
        return {
            statusCode: error.response?.status || 500,
            result: error.response?.data || error.message,
        };
    }
}

export async function getRequest(
    url: string,
    headers?: Record<string, string>, // 修改为普通对象类型
    timeout: number = 5000 // 默认超时时间为 5000 毫秒
): Promise<{ statusCode: number; result: any }> {
    try {
        const response = await axios.get(url, { headers: headers || {}, timeout });
        return { statusCode: response.status, result: response.data };
    } catch (error: any) {
        return {
            statusCode: error.response?.status || 500,
            result: error.response?.data || error.message,
        };
    }
}

// 在 Vue 中引入方式：
// import { postRequest, getRequest } from '@/classes/requests';
// 然后可以在 Vue 组件的 setup 或 methods 中调用这些函数。
// 例如：
// 在 setup 中：
// setup() {
//     const fetchData = async () => {
//         const postResponse = await postRequest(
//             'https://example.com/api', 
//             { key: 'value' }, 
//             { Authorization: 'Bearer token' }, // 示例 headers
//             10000 // 示例超时时间
//         );
//         console.log(postResponse);
//         console.log('POST Result:', postResponse.result); // 后端返回值
//         console.log('POST Status Code:', postResponse.statusCode); // HTTP 状态码
//         const getResponse = await getRequest(
//             'https://example.com/api', 
//             { Authorization: 'Bearer token' }, // 示例 headers
//             10000 // 示例超时时间
//         );
//         console.log(getResponse);
//         console.log('GET Result:', getResponse.result); // 后端返回值
//         console.log('GET Status Code:', getResponse.statusCode); // HTTP 状态码
//     };
//     fetchData();
//     return {};
// }
// 在 methods 中：
// methods: {
//     async fetchData() {
//         const postResponse = await postRequest('https://example.com/api', { key: 'value' });
//         console.log(postResponse);
//         console.log('POST Result:', postResponse.result); // 后端返回值
//         console.log('POST Status Code:', postResponse.statusCode); // HTTP 状态码
//         const getResponse = await getRequest('https://example.com/api');
//         console.log(getResponse);
//         console.log('GET Result:', getResponse.result); // 后端返回值
//         console.log('GET Status Code:', getResponse.statusCode); // HTTP 状态码
//     }
// }
