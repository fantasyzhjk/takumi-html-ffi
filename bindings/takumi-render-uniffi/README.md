# takumi-render-uniffi

### Rust 核心测试

```text
cargo test --manifest-path bindings/takumi-render-uniffi/Cargo.toml
```

### C# smoke

```text
dotnet test bindings/takumi-render-uniffi/csharp/Takumi.Render.UniFFI.Tests/Takumi.Render.UniFFI.Tests.csproj
```

### JVM / Java smoke

```text
gradle -p bindings/takumi-render-uniffi/jvm test
```

## 构建指南

### 构建 C# 库给外部使用

1. 生成 C# 绑定和 native 资源：

```text
python3 bindings/takumi-render-uniffi/scripts/prepare_packaged_bindings.py \
  --language csharp \
  --project-dir bindings/takumi-render-uniffi/csharp/Takumi.Render.UniFFI
```

2. 构建 C# 库：

```text
dotnet build bindings/takumi-render-uniffi/csharp/Takumi.Render.UniFFI/Takumi.Render.UniFFI.csproj -c Release
```

3. 产物位置：

- `bindings/takumi-render-uniffi/csharp/Takumi.Render.UniFFI/bin/Release/net10.0/Takumi.Render.UniFFI.dll`
- 同目录下会带上嵌入的 native 资源

4. 外部项目使用方式：

- 直接 `ProjectReference` 到 `Takumi.Render.UniFFI.csproj`
- 或者把发布后的 `Takumi.Render.UniFFI.dll` 作为普通引用加入项目

### 构建 JVM / Java 库

1. 先生成 Kotlin 绑定和 native 资源：

```text
python3 bindings/takumi-render-uniffi/scripts/prepare_packaged_bindings.py \
  --language kotlin \
  --project-dir bindings/takumi-render-uniffi/jvm
```

2. 构建 jar：

```text
gradle -p bindings/takumi-render-uniffi/jvm jar
```

3. 产物位置：

- `bindings/takumi-render-uniffi/jvm/build/libs/takumi-render-uniffi-jvm-0.1.0.jar`

4. 外部 Java 项目使用方式：

- 把上面的 jar 文件作为文件依赖加入
- 不需要直接引用 Gradle project

### 更新 bindings

只要 Rust 核心 API、UniFFI 类型或方法签名变化，就重新跑下面两个命令：

```text
python3 bindings/takumi-render-uniffi/scripts/prepare_packaged_bindings.py \
  --language csharp \
  --project-dir bindings/takumi-render-uniffi/csharp/Takumi.Render.UniFFI
```

```text
python3 bindings/takumi-render-uniffi/scripts/prepare_packaged_bindings.py \
  --language kotlin \
  --project-dir bindings/takumi-render-uniffi/jvm
```
