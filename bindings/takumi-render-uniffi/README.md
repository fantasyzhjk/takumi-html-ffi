# takumi-render-uniffi

### Rust 测试

```text
cargo test --manifest-path bindings/takumi-render-uniffi/Cargo.toml
```

### C# 测试

```text
dotnet test bindings/takumi-render-uniffi/csharp/Takumi.Render.UniFFI.Tests/Takumi.Render.UniFFI.Tests.csproj
```

### Java 测试

```text
mvn -f bindings/takumi-render-uniffi/jvm/pom.xml test
```

## 构建指南

### 构建 C# 库给外部使用

1. 生成 C# 绑定和 native 资源：

```text
python3 bindings/takumi-render-uniffi/scripts/prepare_packaged_bindings.py --language csharp --project-dir bindings/takumi-render-uniffi/csharp/Takumi.Render.UniFFI
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

### 构建 Java 库

1. 使用 JDK 22+ 构建 jar（Maven 会自动生成 Java binding 和 native 资源）：

```text
mvn -f bindings/takumi-render-uniffi/jvm/pom.xml package
```

2. 产物位置：

- `bindings/takumi-render-uniffi/jvm/target/takumi-render-uniffi-jvm-<version>.jar`

运行 Java 程序时需要添加 `--enable-native-access=ALL-UNNAMED`。

### 更新 bindings

只要 Rust 核心 API、UniFFI 类型或方法签名变化，就重新跑下面两个命令：

```text
python3 bindings/takumi-render-uniffi/scripts/prepare_packaged_bindings.py \
  --language csharp \
  --project-dir bindings/takumi-render-uniffi/csharp/Takumi.Render.UniFFI
```

```text
python3 bindings/takumi-render-uniffi/scripts/prepare_packaged_bindings.py \
  --language java \
  --project-dir bindings/takumi-render-uniffi/jvm
```
