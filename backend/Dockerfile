# ============================================================
# Rust Backend Dockerfile
# 多阶段构建：编译阶段 + 运行阶段
# ============================================================

# ---------- 编译阶段 ----------
FROM rust:1.81-bookworm AS builder

WORKDIR /app

# 安装系统依赖
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# 先创建空项目以缓存依赖
RUN cargo init --name lacquer-monitor-backend

# 复制 Cargo.toml 和 Cargo.lock
COPY backend/Cargo.toml ./
COPY backend/Cargo.lock ./

# 构建依赖缓存（使用 dummy main.rs）
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release 2>&1 || true

# 复制源代码
COPY backend/src ./src

# 构建正式版本
RUN cargo build --release

# ---------- 运行阶段 ----------
FROM debian:bookworm-slim

WORKDIR /app

# 安装运行时依赖
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    curl \
    tzdata \
    && rm -rf /var/lib/apt/lists/*

# 设置时区
ENV TZ=Asia/Shanghai
RUN ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ > /etc/timezone

# 复制编译产物
COPY --from=builder /app/target/release/lacquer-monitor-backend /app/backend

# 前端静态文件通过 docker-compose 挂载到 /app/frontend
ENV STATIC_DIR=/app/frontend

# 暴露端口
EXPOSE 8080

# 健康检查
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8080/api/statistics || exit 1

# 启动命令
CMD ["/app/backend"]
