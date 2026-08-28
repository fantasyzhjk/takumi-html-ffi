package io.github.zhjk.takumi.render.uniffi;

import java.io.File;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.util.concurrent.atomic.AtomicBoolean;

public final class TakumiRenderUniffiNativeLoader {
    private static final AtomicBoolean LOADED = new AtomicBoolean();

    private TakumiRenderUniffiNativeLoader() {}

    public static void ensureLoaded() {
        if (LOADED.get()) {
            return;
        }

        synchronized (TakumiRenderUniffiNativeLoader.class) {
            if (LOADED.get()) {
                return;
            }

            String rid = currentRid();
            String fileName = nativeFileName();
            String resourcePath = "/native/" + rid + "/" + fileName;
            Path targetPath = extractionPath(rid, fileName);

            try (var input = TakumiRenderUniffiNativeLoader.class.getResourceAsStream(resourcePath)) {
                if (input == null) {
                    throw new IllegalStateException(
                            "Embedded native library resource not found: " + resourcePath);
                }
                Files.createDirectories(targetPath.getParent());
                Files.copy(input, targetPath, StandardCopyOption.REPLACE_EXISTING);
            } catch (IOException e) {
                throw new IllegalStateException("Failed to extract native library: " + resourcePath, e);
            }

            targetPath.toFile().deleteOnExit();
            System.setProperty(
                    "uniffi.component.takumi_render_uniffi.libraryOverride",
                    targetPath.toAbsolutePath().toString());
            UniffiInitializer.ensureInitialized();
            LOADED.set(true);
        }
    }

    private static Path extractionPath(String rid, String fileName) {
        return new File(
                        new File(
                                new File(System.getProperty("java.io.tmpdir"), "takumi-render-uniffi"),
                                implementationVersion()),
                        rid + "/" + fileName)
                .toPath();
    }

    private static String implementationVersion() {
        String version = TakumiRenderUniffiNativeLoader.class.getPackage().getImplementationVersion();
        return version == null ? "0.1.0" : version;
    }

    private static String currentRid() {
        String os = System.getProperty("os.name").toLowerCase();
        String arch = System.getProperty("os.arch").toLowerCase();

        String normalizedOs;
        if (os.contains("win")) {
            normalizedOs = "win";
        } else if (os.contains("mac") || os.contains("darwin")) {
            normalizedOs = "osx";
        } else if (os.contains("nux") || os.contains("linux")) {
            normalizedOs = "linux";
        } else {
            throw new IllegalStateException("Unsupported operating system: " + os);
        }

        String normalizedArch;
        if (arch.equals("x86_64") || arch.equals("amd64")) {
            normalizedArch = "x64";
        } else if (arch.equals("aarch64") || arch.equals("arm64")) {
            normalizedArch = "arm64";
        } else {
            throw new IllegalStateException("Unsupported CPU architecture: " + arch);
        }

        return normalizedOs + "-" + normalizedArch;
    }

    private static String nativeFileName() {
        String os = System.getProperty("os.name").toLowerCase();
        if (os.contains("win")) {
            return "takumi_render_uniffi.dll";
        }
        if (os.contains("mac") || os.contains("darwin")) {
            return "libtakumi_render_uniffi.dylib";
        }
        if (os.contains("nux") || os.contains("linux")) {
            return "libtakumi_render_uniffi.so";
        }
        throw new IllegalStateException("Unsupported operating system: " + os);
    }
}
