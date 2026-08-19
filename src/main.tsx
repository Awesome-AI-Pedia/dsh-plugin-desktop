import ReactDOM from 'react-dom/client'
import { App } from './app'

// 注：故意不用 React.StrictMode。StrictMode 会在 dev 下重复触发 useEffect，
// 导致 launch_harness 被并发调用两次，一次成功一次可能在锁上失败并覆盖状态。
ReactDOM.createRoot(document.getElementById('root')!).render(<App />)
