# ============================================================
# NB-IoT 模拟器 Dockerfile
# ============================================================

FROM python:3.11-slim-bookworm

WORKDIR /app

# 设置时区
ENV TZ=Asia/Shanghai
RUN ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ > /etc/timezone

# 安装系统依赖
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    && rm -rf /var/lib/apt/lists/*

# 复制 requirements.txt
COPY simulator/requirements.txt ./

# 安装 Python 依赖
RUN pip install --no-cache-dir -r requirements.txt

# 复制模拟器代码
COPY simulator/nb_iot_simulator.py ./

# 暴露控制端口
EXPOSE 8081

# 健康检查
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8081/health || exit 1

# 启动命令
CMD ["python", "nb_iot_simulator.py"]
