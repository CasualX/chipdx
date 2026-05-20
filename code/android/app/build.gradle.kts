buildscript {
	repositories {
		google()
		mavenCentral()
	}
	dependencies {
		classpath("org.apache.xmlgraphics:batik-all:1.19")
	}
}

import java.awt.Color
import java.awt.RenderingHints
import java.awt.image.BufferedImage
import java.util.Properties
import javax.imageio.ImageIO
import org.apache.batik.transcoder.SVGAbstractTranscoder
import org.apache.batik.transcoder.TranscoderInput
import org.apache.batik.transcoder.image.ImageTranscoder
import org.gradle.api.DefaultTask
import org.gradle.api.file.DirectoryProperty
import org.gradle.api.file.RegularFileProperty
import org.gradle.api.tasks.InputFile
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.TaskAction
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
	id("com.android.application")
	id("org.jetbrains.kotlin.android")
}

val repoRoot = rootProject.projectDir.resolve("../..").canonicalFile
val rustJniLibsDir = layout.buildDirectory.dir("rustJniLibs")
val generatedIconResDir = layout.buildDirectory.dir("generated/icon-res")

fun requiredStringGradleProperty(name: String): String {
	return providers.gradleProperty(name).orNull
		?: throw GradleException("Missing required Gradle property: $name")
}

fun requiredIntGradleProperty(name: String): Int {
	return requiredStringGradleProperty(name).toIntOrNull()
		?: throw GradleException("Gradle property $name must be an integer")
}

val androidCompileSdk = requiredIntGradleProperty("chipdx.android.compileSdk")
val androidTargetSdk = requiredIntGradleProperty("chipdx.android.targetSdk")
val androidBuildToolsVersion = requiredStringGradleProperty("chipdx.android.buildToolsVersion")
val androidNdkVersion = requiredStringGradleProperty("chipdx.android.ndkVersion")

abstract class GenerateAndroidLauncherIconsTask : DefaultTask() {
	@get:InputFile
	abstract val sourceSvg: RegularFileProperty

	@get:OutputDirectory
	abstract val outputDir: DirectoryProperty

	@TaskAction
	fun generate() {
		val outputRoot = outputDir.get().asFile
		if (outputRoot.exists() && !outputRoot.deleteRecursively()) {
			throw GradleException("Failed to delete $outputRoot")
		}
		outputRoot.mkdirs()

		val drawableDir = outputRoot.resolve("drawable-nodpi")
		val anydpiDir = outputRoot.resolve("mipmap-anydpi-v26")
		val legacyDirs = listOf("mdpi", "hdpi", "xhdpi", "xxhdpi", "xxxhdpi").associateWith {
			outputRoot.resolve("mipmap-$it")
		}

		listOf(drawableDir, anydpiDir, *legacyDirs.values.toTypedArray()).forEach { it.mkdirs() }

		anydpiDir.resolve("ic_launcher.xml").writeText(
			"""
				<?xml version="1.0" encoding="utf-8"?>
				<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
					<background android:drawable="@drawable/ic_launcher_adaptive" />
					<foreground android:drawable="@android:color/transparent" />
				</adaptive-icon>
			""".trimIndent() + "\n",
		)
		anydpiDir.resolve("ic_launcher_round.xml").writeText(
			"""
				<?xml version="1.0" encoding="utf-8"?>
				<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
					<background android:drawable="@drawable/ic_launcher_adaptive" />
					<foreground android:drawable="@android:color/transparent" />
				</adaptive-icon>
			""".trimIndent() + "\n",
		)

		writePng(composeAdaptiveIcon(), drawableDir.resolve("ic_launcher_adaptive.png"))
		legacyDirs.forEach { (density, dir) ->
			val size = when (density) {
				"mdpi" -> 48
				"hdpi" -> 72
				"xhdpi" -> 96
				"xxhdpi" -> 144
				else -> 192
			}
			val icon = renderSvg(size, size)
			writePng(icon, dir.resolve("ic_launcher.png"))
			writePng(icon, dir.resolve("ic_launcher_round.png"))
		}
	}

	private fun composeAdaptiveIcon(): BufferedImage {
		val canvas = BufferedImage(432, 432, BufferedImage.TYPE_INT_ARGB)
		val graphics = canvas.createGraphics()
		try {
			graphics.color = Color(0x02, 0x52, 0x5b)
			graphics.fillRect(0, 0, 432, 432)
			val foreground = renderSvg(288, 288)
			graphics.setRenderingHint(RenderingHints.KEY_INTERPOLATION, RenderingHints.VALUE_INTERPOLATION_BICUBIC)
			graphics.drawImage(foreground, 72, 72, null)
		}
		finally {
			graphics.dispose()
		}
		return canvas
	}

	private fun renderSvg(width: Int, height: Int): BufferedImage {
		val transcoder = object : ImageTranscoder() {
			private var image: BufferedImage? = null

			override fun createImage(imageWidth: Int, imageHeight: Int): BufferedImage {
				return BufferedImage(imageWidth, imageHeight, BufferedImage.TYPE_INT_ARGB)
			}

			override fun writeImage(image: BufferedImage, output: org.apache.batik.transcoder.TranscoderOutput?) {
				this.image = image
			}

			fun output(): BufferedImage {
				return checkNotNull(image) { "SVG rasterization did not produce an image" }
			}
		}

		transcoder.addTranscodingHint(SVGAbstractTranscoder.KEY_WIDTH, width.toFloat())
		transcoder.addTranscodingHint(SVGAbstractTranscoder.KEY_HEIGHT, height.toFloat())
		transcoder.transcode(TranscoderInput(sourceSvg.get().asFile.toURI().toString()), null)
		return transcoder.output()
	}

	private fun writePng(image: BufferedImage, outputFile: java.io.File) {
		ImageIO.write(image, "png", outputFile)
	}
}

fun configuredAndroidSdkRoot(): String {
	val envSdk = System.getenv("ANDROID_HOME") ?: System.getenv("ANDROID_SDK_ROOT")
	if (!envSdk.isNullOrBlank()) {
		return envSdk
	}
	val localProperties = rootProject.file("local.properties")
	if (localProperties.isFile) {
		val props = Properties()
		localProperties.inputStream().use(props::load)
		val sdkDir = props.getProperty("sdk.dir")
		if (!sdkDir.isNullOrBlank()) {
			return sdkDir
		}
	}
	throw GradleException("Android SDK location is not configured. Set ANDROID_HOME or ANDROID_SDK_ROOT, or open code/android in Android Studio once so it writes local.properties.")
}

dependencies {
	implementation("androidx.core:core-ktx:1.18.0")
}

val buildRustAndroid by tasks.registering(Exec::class) {
	group = "build"
	description = "Compile chipjni for Android and stage the JNI libraries for packaging."
	workingDir = repoRoot
	outputs.upToDateWhen { false }
	commandLine(
		"cargo", "ndk",
		"-t", "arm64-v8a",
		"-t", "x86_64",
		"-o", rustJniLibsDir.get().asFile.absolutePath,
		"build", "--release",
		"-p", "chipjni",
	)

	doFirst {
		val sdkRoot = configuredAndroidSdkRoot()
		val ndkRoot = file("$sdkRoot/ndk/${android.ndkVersion}")
		if (!ndkRoot.isDirectory) {
			throw GradleException("Android NDK not found at $ndkRoot. Install NDK ${android.ndkVersion} from Android Studio's SDK Manager or update android.ndkVersion.")
		}
		rustJniLibsDir.get().asFile.mkdirs()
		environment("ANDROID_HOME", sdkRoot)
		environment("ANDROID_SDK_ROOT", sdkRoot)
		environment("ANDROID_NDK_HOME", ndkRoot.absolutePath)
	}
}

val generateAndroidLauncherIcons by tasks.registering(GenerateAndroidLauncherIconsTask::class) {
	group = "build"
	description = "Generate Android launcher icon resources from code/android/icon.svg."
	sourceSvg.set(rootProject.file("icon.svg"))
	outputDir.set(generatedIconResDir)
}

android {
	namespace = "net.casualhacks.chipdx"
	buildToolsVersion = androidBuildToolsVersion
	compileSdk = androidCompileSdk
	ndkVersion = androidNdkVersion

	defaultConfig {
		applicationId = "net.casualhacks.chipdx"
		minSdk = 26
		targetSdk = androidTargetSdk
		versionCode = 1
		versionName = "1.0"
	}

	buildTypes {
		debug {
			isMinifyEnabled = false
		}
		release {
			isMinifyEnabled = false
			proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
		}
	}

	compileOptions {
		sourceCompatibility = JavaVersion.VERSION_17
		targetCompatibility = JavaVersion.VERSION_17
	}

	sourceSets.named("main") {
		jniLibs.setSrcDirs(listOf(rustJniLibsDir))
		res.srcDir(generatedIconResDir)
	}
}

kotlin {
	compilerOptions {
		jvmTarget = JvmTarget.JVM_17
	}
}

tasks.named("preBuild") {
	dependsOn(generateAndroidLauncherIcons)
	dependsOn(buildRustAndroid)
}
