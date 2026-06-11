#!/usr/bin/env python3
"""
NB-IoT 数据模拟器
模拟50件漆器的50台传感器（30台含水率 + 20台应变片）每小时上报数据
支持通过环境变量配置，支持快速失水注入
"""

import requests
import random
import time
import json
import os
import threading
from datetime import datetime, timezone, timedelta
from typing import List, Dict, Optional
from http.server import HTTPServer, BaseHTTPRequestHandler
from argparse import ArgumentParser

API_BASE_URL = os.getenv("API_BASE_URL", "http://backend:8080/api")
REPORT_INTERVAL = int(os.getenv("REPORT_INTERVAL", "3600"))
NUM_WARES = int(os.getenv("NUM_WARES", "50"))
BATCH_SIZE = int(os.getenv("BATCH_SIZE", "50"))
CONTROL_PORT = int(os.getenv("CONTROL_PORT", "8081"))

class FastDehydrationInjector:
    def __init__(self):
        self.active = False
        self.target_sensor_ids = []
        self.drop_rate_multiplier = 5.0
        self.duration_hours = 0
        self.lock = threading.Lock()

    def trigger(self, sensor_ids: Optional[List[str]] = None, multiplier: float = 5.0, duration_hours: float = 2.0):
        with self.lock:
            self.active = True
            self.target_sensor_ids = sensor_ids or []
            self.drop_rate_multiplier = multiplier
            self.duration_hours = duration_hours
            self.start_time = datetime.now(timezone.utc)

    def stop(self):
        with self.lock:
            self.active = False

    def get_drop_multiplier(self, sensor_id: str) -> float:
        with self.lock:
            if not self.active:
                return 1.0
            elapsed = (datetime.now(timezone.utc) - self.start_time).total_seconds() / 3600.0
            if elapsed > self.duration_hours:
                self.active = False
                return 1.0
            if not self.target_sensor_ids or sensor_id in self.target_sensor_ids:
                return self.drop_rate_multiplier
            return 1.0

    def status(self) -> Dict:
        with self.lock:
            elapsed = 0.0
            remaining = 0.0
            if hasattr(self, 'start_time') and self.active:
                elapsed = (datetime.now(timezone.utc) - self.start_time).total_seconds() / 3600.0
                remaining = max(0, self.duration_hours - elapsed)
            return {
                "active": self.active,
                "target_sensors": self.target_sensor_ids,
                "drop_rate_multiplier": self.drop_rate_multiplier,
                "duration_hours": self.duration_hours,
                "elapsed_hours": round(elapsed, 2),
                "remaining_hours": round(remaining, 2)
            }


class NbIoTSimulator:
    def __init__(self, api_base_url: str = API_BASE_URL, num_wares: int = NUM_WARES):
        self.api_base_url = api_base_url
        self.num_wares = num_wares
        self.moisture_sensors = []
        self.strain_sensors = []
        self.moisture_history = {}
        self.strain_history = {}
        self.signal_base = {}
        self.sensor_to_ware = {}
        self.dehydration_injector = FastDehydrationInjector()
        self._init_sensors()

    def _init_sensors(self):
        num_moisture_per_ware = max(1, 30 // self.num_wares)
        num_strain_per_ware = max(1, 20 // self.num_wares)

        moisture_count = 0
        for ware_id in range(1, self.num_wares + 1):
            for i in range(num_moisture_per_ware):
                if moisture_count >= 30:
                    break
                device_id = f"MS{moisture_count + 1:04d}"
                self.moisture_sensors.append(device_id)
                self.moisture_history[device_id] = 75.0 + random.uniform(-5, 10)
                self.signal_base[device_id] = random.uniform(-65, -80)
                self.sensor_to_ware[device_id] = ware_id
                moisture_count += 1

        strain_count = 0
        for ware_id in range(1, self.num_wares + 1):
            for i in range(num_strain_per_ware):
                if strain_count >= 20:
                    break
                device_id = f"SS{strain_count + 1:04d}"
                self.strain_sensors.append(device_id)
                self.strain_history[device_id] = 0.5 + random.uniform(-0.2, 1.0)
                self.signal_base[device_id] = random.uniform(-65, -80)
                self.sensor_to_ware[device_id] = ware_id
                strain_count += 1

    def _simulate_signal_attenuation(self, device_id: str) -> float:
        base = self.signal_base.get(device_id, -70.0)
        rain_attenuation = random.uniform(0, 5) if random.random() < 0.3 else 0
        distance_attenuation = random.uniform(0, 3)
        obstacle_attenuation = random.uniform(5, 15) if random.random() < 0.1 else 0
        fading = random.gauss(0, 2)
        signal = base - rain_attenuation - distance_attenuation - obstacle_attenuation + fading
        return round(max(-120.0, min(-40.0, signal)), 1)

    def generate_moisture_data(self, device_id: str) -> Dict:
        prev_value = self.moisture_history.get(device_id, 75.0)

        base_drop_rate = random.uniform(0.02, 0.15)
        drop_multiplier = self.dehydration_injector.get_drop_multiplier(device_id)
        drop_rate = base_drop_rate * drop_multiplier
        noise = random.uniform(-0.5, 0.5)

        new_value = prev_value - drop_rate + noise
        new_value = max(5.0, min(95.0, new_value))

        if random.random() < 0.02:
            anomaly_drop = random.uniform(12, 25) * drop_multiplier
            new_value = prev_value - anomaly_drop
            new_value = max(5.0, min(95.0, new_value))
            print(f"⚠️  模拟异常: {device_id} 含水率突降至 {new_value:.1f}% (快速失水倍率: {drop_multiplier:.1f}x)")

        self.moisture_history[device_id] = new_value

        return {
            "device_id": device_id,
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "sensor_type": "moisture",
            "value": round(new_value, 2),
            "temperature": round(20 + random.uniform(-3, 5), 1),
            "battery_level": round(80 + random.uniform(-5, 15), 1),
            "signal_strength": self._simulate_signal_attenuation(device_id)
        }

    def generate_strain_data(self, device_id: str) -> Dict:
        prev_value = self.strain_history.get(device_id, 1.0)

        increase_rate = random.uniform(0.005, 0.03)
        noise = random.uniform(-0.1, 0.1)

        new_value = prev_value + increase_rate + noise
        new_value = max(0.1, min(15.0, new_value))

        if random.random() < 0.015:
            new_value = random.uniform(6, 12)
            print(f"⚠️  模拟异常: {device_id} 收缩应变升至 {new_value:.2f}%")

        self.strain_history[device_id] = new_value

        return {
            "device_id": device_id,
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "sensor_type": "strain",
            "value": round(new_value, 3),
            "temperature": round(20 + random.uniform(-3, 5), 1),
            "battery_level": round(80 + random.uniform(-5, 15), 1),
            "signal_strength": self._simulate_signal_attenuation(device_id)
        }

    def send_batch(self, batch_data: List[Dict]) -> bool:
        try:
            response = requests.post(
                f"{self.api_base_url}/nb-iot/batch",
                json={"packets": batch_data},
                timeout=30
            )
            if response.status_code == 200:
                result = response.json()
                return result.get("success", False)
            else:
                print(f"❌ 批量发送失败: HTTP {response.status_code}")
                return False
        except Exception as e:
            print(f"❌ 批量发送异常: {e}")
            return False

    def run_cycle(self):
        print(f"\n{'='*60}")
        print(f"📡 NB-IoT 数据上报周期 - {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
        print(f"{'='*60}")
        print(f"📦 漆器数量: {self.num_wares} 件")
        print(f"💧 含水率传感器: {len(self.moisture_sensors)} 台")
        print(f"📏 应变传感器: {len(self.strain_sensors)} 台")

        dehyd_status = self.dehydration_injector.status()
        if dehyd_status["active"]:
            print(f"� 快速失水注入: 激活中 | 倍率: {dehyd_status['drop_rate_multiplier']}x | 剩余: {dehyd_status['remaining_hours']}h")

        all_data = []
        success_count = 0
        fail_count = 0

        print(f"\n💧 生成含水率数据:")
        for i, device_id in enumerate(self.moisture_sensors):
            data = self.generate_moisture_data(device_id)
            all_data.append(data)
            if i < 3:
                ware_id = self.sensor_to_ware.get(device_id, "?")
                print(f"  ✅ {device_id} (漆器#{ware_id}): {data['value']:.2f}%")

        print(f"\n📏 生成应变数据:")
        for i, device_id in enumerate(self.strain_sensors):
            data = self.generate_strain_data(device_id)
            all_data.append(data)
            if i < 3:
                ware_id = self.sensor_to_ware.get(device_id, "?")
                print(f"  ✅ {device_id} (漆器#{ware_id}): {data['value']:.3f}%")

        print(f"\n📤 批量发送 {len(all_data)} 条数据...")
        for i in range(0, len(all_data), BATCH_SIZE):
            batch = all_data[i:i + BATCH_SIZE]
            if self.send_batch(batch):
                success_count += len(batch)
                print(f"  ✅ 批次 {i // BATCH_SIZE + 1}: {len(batch)} 条")
            else:
                fail_count += len(batch)
                print(f"  ❌ 批次 {i // BATCH_SIZE + 1}: {len(batch)} 条")

        print(f"\n📊 本次上报统计:")
        print(f"   成功: {success_count} 条")
        print(f"   失败: {fail_count} 条")
        print(f"   总计: {success_count + fail_count} 条")

    def run_continuous(self, interval_seconds: int = 3600):
        print(f"🚀 NB-IoT 模拟器启动")
        print(f"   上报间隔: {interval_seconds} 秒 ({interval_seconds/3600:.1f} 小时)")
        print(f"   API地址: {self.api_base_url}")
        print(f"   漆器数量: {self.num_wares} 件")
        print(f"   含水率传感器: {len(self.moisture_sensors)} 台")
        print(f"   应变传感器: {len(self.strain_sensors)} 台")
        print(f"   控制端口: {CONTROL_PORT}")
        print(f"   批量大小: {BATCH_SIZE}")

        try:
            while True:
                cycle_start = time.time()
                self.run_cycle()
                elapsed = time.time() - cycle_start
                sleep_time = max(0, interval_seconds - elapsed)
                print(f"\n⏳ 等待 {sleep_time:.1f} 秒后进行下一次上报...")
                time.sleep(sleep_time)
        except KeyboardInterrupt:
            print("\n\n👋 模拟器已停止")

    def run_once(self):
        self.run_cycle()


class ControlHandler(BaseHTTPRequestHandler):
    simulator = None

    def do_GET(self):
        if self.path == "/health":
            self.send_json_response(200, {"status": "ok", "timestamp": datetime.now(timezone.utc).isoformat()})
        elif self.path == "/status":
            status = self.simulator.dehydration_injector.status()
            self.send_json_response(200, status)
        elif self.path == "/metrics":
            self.send_json_response(200, {
                "moisture_sensors": len(self.simulator.moisture_sensors),
                "strain_sensors": len(self.simulator.strain_sensors),
                "num_wares": self.simulator.num_wares,
                "moisture_history": {k: round(v, 2) for k, v in self.simulator.moisture_history.items()},
                "strain_history": {k: round(v, 3) for k, v in self.simulator.strain_history.items()}
            })
        else:
            self.send_json_response(404, {"error": "Not found"})

    def do_POST(self):
        if self.path == "/inject-fast-dehydration":
            try:
                content_length = int(self.headers['Content-Length'])
                post_data = self.rfile.read(content_length)
                data = json.loads(post_data)
                sensor_ids = data.get("sensor_ids")
                multiplier = data.get("multiplier", 5.0)
                duration_hours = data.get("duration_hours", 2.0)
                self.simulator.dehydration_injector.trigger(sensor_ids, multiplier, duration_hours)
                self.send_json_response(200, {
                    "status": "success",
                    "message": f"快速失水已激活",
                    "details": self.simulator.dehydration_injector.status()
                })
            except Exception as e:
                self.send_json_response(400, {"error": str(e)})
        elif self.path == "/stop-fast-dehydration":
            self.simulator.dehydration_injector.stop()
            self.send_json_response(200, {"status": "success", "message": "快速失水已停止"})
        else:
            self.send_json_response(404, {"error": "Not found"})

    def send_json_response(self, status_code: int, data: Dict):
        self.send_response(status_code)
        self.send_header('Content-type', 'application/json')
        self.end_headers()
        self.wfile.write(json.dumps(data).encode('utf-8'))

    def log_message(self, format, *args):
        pass


def start_control_server(simulator: NbIoTSimulator):
    ControlHandler.simulator = simulator
    server = HTTPServer(("0.0.0.0", CONTROL_PORT), ControlHandler)
    print(f"🎛️  控制服务器启动在端口 {CONTROL_PORT}")
    print(f"   POST /inject-fast-dehydration - 触发快速失水")
    print(f"   POST /stop-fast-dehydration - 停止快速失水")
    print(f"   GET /status - 查看状态")
    print(f"   GET /health - 健康检查")
    print(f"   GET /metrics - 查看传感器数据")
    server.serve_forever()


def main():
    parser = ArgumentParser(description="NB-IoT 数据模拟器")
    parser.add_argument("--url", default=API_BASE_URL, help="API 基础地址")
    parser.add_argument("--interval", type=int, default=REPORT_INTERVAL, help="上报间隔（秒）")
    parser.add_argument("--once", action="store_true", help="只运行一次")
    parser.add_argument("--num-wares", type=int, default=NUM_WARES, help="漆器数量")
    parser.add_argument("--fast", action="store_true", help="快速模式，10秒上报一次")

    args = parser.parse_args()

    interval = 10 if args.fast else args.interval

    simulator = NbIoTSimulator(api_base_url=args.url, num_wares=args.num_wares)

    control_thread = threading.Thread(target=start_control_server, args=(simulator,), daemon=True)
    control_thread.start()

    time.sleep(0.5)

    if args.once:
        simulator.run_once()
    else:
        simulator.run_continuous(interval_seconds=interval)


if __name__ == "__main__":
    main()
