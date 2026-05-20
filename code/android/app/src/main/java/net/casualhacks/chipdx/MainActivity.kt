package net.casualhacks.chipdx

import android.app.Activity
import android.content.res.ColorStateList
import android.graphics.Color
import android.graphics.Rect
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.graphics.drawable.StateListDrawable
import android.os.Build
import android.os.Bundle
import android.view.Gravity
import android.view.KeyEvent
import android.view.MotionEvent
import android.view.View
import android.view.ViewGroup
import android.view.WindowManager
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView
import androidx.annotation.Keep
import androidx.core.view.WindowCompat
import java.io.File
import kotlin.math.abs
import kotlin.math.min

class MainActivity : Activity() {
	private data class TouchButton(val view: View, val bit: Int)
	private data class DirectionPad(
		val view: View,
		val up: View,
		val left: View,
		val down: View,
		val right: View,
	)

	private var nativeHandle: Long = 0
	@Volatile
	private var keyButtonsMask: Int = 0
	@Volatile
	private var touchButtonsMask: Int = 0
	private lateinit var surfaceView: GameSurfaceView
	private lateinit var audioBank: AudioBank
	private val touchButtons = mutableListOf<TouchButton>()
	private var directionPad: DirectionPad? = null

	override fun onCreate(savedInstanceState: Bundle?) {
		super.onCreate(savedInstanceState)
		window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
		audioBank = AudioBank(this)
		nativeHandle = ChipJNI.nativeCreate(this)
		check(nativeHandle != 0L) { "Failed to create native game host" }
		ChipJNI.nativeAudioInit(nativeHandle)

		surfaceView = GameSurfaceView(this, nativeHandle) { keyButtonsMask or touchButtonsMask }
		setContentView(buildContentView())
		enterImmersiveMode()
	}

	override fun onResume() {
		super.onResume()
		surfaceView.onResume()
		audioBank.onResume()
		enterImmersiveMode()
	}

	override fun onPause() {
		clearAllInputs()
		audioBank.onPause()
		surfaceView.onPause()
		super.onPause()
	}

	override fun onDestroy() {
		clearAllInputs()
		if (nativeHandle != 0L) {
			ChipJNI.nativeDestroy(nativeHandle)
			nativeHandle = 0
		}
		audioBank.release()
		super.onDestroy()
	}

	override fun onWindowFocusChanged(hasFocus: Boolean) {
		super.onWindowFocusChanged(hasFocus)
		if (hasFocus) {
			enterImmersiveMode()
		}
		else {
			clearAllInputs()
		}
	}

	override fun onKeyDown(keyCode: Int, event: KeyEvent): Boolean {
		mapKey(keyCode)?.let {
			setKeyBit(it, true)
			return true
		}
		return super.onKeyDown(keyCode, event)
	}

	override fun onKeyUp(keyCode: Int, event: KeyEvent): Boolean {
		mapKey(keyCode)?.let {
			setKeyBit(it, false)
			return true
		}
		return super.onKeyUp(keyCode, event)
	}

	@Keep
	@Suppress("unused")
	fun registerSound(id: Int, relativePath: String, data: ByteArray) {
		audioBank.registerSound(id, relativePath, data)
	}

	@Keep
	@Suppress("unused")
	fun registerMusic(id: Int, relativePath: String, data: ByteArray) {
		audioBank.registerMusic(id, relativePath, data)
	}

	@Keep
	@Suppress("unused")
	fun playSound(id: Int) {
		audioBank.playSound(id)
	}

	@Keep
	@Suppress("unused")
	fun playMusic(id: Int) {
		audioBank.playMusic(id)
	}

	@Keep
	@Suppress("unused")
	fun setNativeTitle(title: String) {
		runOnUiThread {
			this.title = title
		}
	}

	@Keep
	@Suppress("unused")
	fun quitGame() {
		runOnUiThread {
			finishAndRemoveTask()
		}
	}

	@Keep
	@Suppress("unused")
	fun saveFile(relativePath: String, data: ByteArray): Boolean {
		return runCatching {
			val target = resolveStoragePath(relativePath)
			target.parentFile?.mkdirs()
			target.writeBytes(data)
		}.isSuccess
	}

	@Keep
	@Suppress("unused")
	fun loadFile(relativePath: String): ByteArray? {
		val target = resolveStoragePath(relativePath)
		return if (target.exists()) target.readBytes() else null
	}

	private fun buildContentView(): View {
		return FrameLayout(this).apply {
			setBackgroundColor(Color.BLACK)
			addView(surfaceView, FrameLayout.LayoutParams(
				ViewGroup.LayoutParams.MATCH_PARENT,
				ViewGroup.LayoutParams.MATCH_PARENT,
			))
			addView(buildControls(), FrameLayout.LayoutParams(
				ViewGroup.LayoutParams.MATCH_PARENT,
				ViewGroup.LayoutParams.MATCH_PARENT,
			))
		}
	}

	private fun buildControls(): View {
		val padSize = dp(220)
		return object : FrameLayout(this) {
			override fun onInterceptTouchEvent(event: MotionEvent): Boolean {
				return true
			}

			override fun onTouchEvent(event: MotionEvent): Boolean {
				updateTouchControls(this, event)
				if (event.actionMasked == MotionEvent.ACTION_UP) {
					performClick()
				}
				return true
			}

			override fun performClick(): Boolean {
				super.performClick()
				return true
			}
		}.apply {
			setPadding(dp(18), dp(18), dp(18), dp(18))
			clipChildren = false
			clipToPadding = false
			addView(buildDirectionalPad(), FrameLayout.LayoutParams(
				padSize,
				padSize,
				Gravity.START or Gravity.BOTTOM,
			))
			addView(buildActionCluster(), FrameLayout.LayoutParams(
				ViewGroup.LayoutParams.WRAP_CONTENT,
				ViewGroup.LayoutParams.WRAP_CONTENT,
				Gravity.END or Gravity.BOTTOM,
			))
		}
	}

	private fun buildDirectionalPad(): View {
		val dirSize = dp(72)
		val inset = dp(16)
		val up = createPadDirection("▲")
		val down = createPadDirection("▼")
		val left = createPadDirection("◀")
		val right = createPadDirection("▶")

		return FrameLayout(this).apply {
			clipChildren = false
			clipToPadding = false
			addView(up, FrameLayout.LayoutParams(dirSize, dirSize, Gravity.TOP or Gravity.CENTER_HORIZONTAL).apply {
				topMargin = inset
			})
			addView(down, FrameLayout.LayoutParams(dirSize, dirSize, Gravity.BOTTOM or Gravity.CENTER_HORIZONTAL).apply {
				bottomMargin = inset
			})
			addView(left, FrameLayout.LayoutParams(dirSize, dirSize, Gravity.START or Gravity.CENTER_VERTICAL).apply {
				leftMargin = inset
			})
			addView(right, FrameLayout.LayoutParams(dirSize, dirSize, Gravity.END or Gravity.CENTER_VERTICAL).apply {
				rightMargin = inset
			})
			addView(View(this@MainActivity), FrameLayout.LayoutParams(dp(28), dp(28), Gravity.CENTER).apply {
				setMargins(dp(20), dp(20), dp(20), dp(20))
			})
			directionPad = DirectionPad(this, up, left, down, right)
		}
	}

	private fun buildActionCluster(): View {
		return LinearLayout(this).apply {
			orientation = LinearLayout.VERTICAL
			gravity = Gravity.END
			clipChildren = false
			clipToPadding = false
			addView(buttonRow("B" to INPUT_B, "A" to INPUT_A))
			addView(createActionButton("Start", INPUT_START, dp(154), dp(56)).withLayoutMargins(dp(10), dp(12), dp(10), 0))
			addView(createActionButton("Select", INPUT_SELECT, dp(154), dp(56)).withLayoutMargins(dp(10), dp(10), dp(10), 0))
		}
	}

	private fun buttonRow(left: Pair<String, Int>, right: Pair<String, Int>): View {
		return LinearLayout(this).apply {
			orientation = LinearLayout.HORIZONTAL
			gravity = Gravity.END
			clipChildren = false
			clipToPadding = false
			addView(createActionButton(left.first, left.second, dp(72), dp(72)).withLayoutMargins(dp(10), dp(4), dp(10), 0))
			addView(createActionButton(right.first, right.second, dp(72), dp(72)).withLayoutMargins(0, dp(4), dp(10), 0))
		}
	}

	private fun createPadDirection(label: String): View {
		return FrameLayout(this).apply {
			rotation = 45f
			background = createDiamondBackground()
			isClickable = false
			isFocusable = false
			addView(createControlLabel(label, 17f, dp(8).toFloat()).apply {
				background = null
				isDuplicateParentStateEnabled = true
				rotation = -45f
			}, FrameLayout.LayoutParams(
				ViewGroup.LayoutParams.MATCH_PARENT,
				ViewGroup.LayoutParams.MATCH_PARENT,
				Gravity.CENTER,
			))
		}
	}

	private fun createActionButton(label: String, bit: Int, width: Int, height: Int): View {
		return createControlLabel(label, 13f, dp(16).toFloat()).apply {
			layoutParams = LinearLayout.LayoutParams(width, height)
			touchButtons += TouchButton(this, bit)
		}
	}

	private fun createControlLabel(label: String, textSizeSp: Float, cornerRadius: Float): TextView {
		return TextView(this).apply {
			text = label
			gravity = Gravity.CENTER
			setTypeface(typeface, Typeface.BOLD)
			textSize = textSizeSp
			letterSpacing = 0.04f
			setTextColor(controlTextColors())
			background = createControlBackground(cornerRadius)
			isClickable = false
			isFocusable = false
			includeFontPadding = false
		}
	}

	private fun createControlBackground(cornerRadius: Float): StateListDrawable {
		return StateListDrawable().apply {
			addState(intArrayOf(android.R.attr.state_pressed), createRoundedRect(
				fillColor = Color.argb(160, 86, 198, 243),
				strokeColor = Color.argb(192, 145, 224, 255),
				cornerRadius = cornerRadius,
			))
			addState(intArrayOf(), createRoundedRect(
				fillColor = Color.argb(80, 15, 23, 42),
				strokeColor = Color.argb(72, 148, 163, 184),
				cornerRadius = cornerRadius,
			))
		}
	}

	private fun createDiamondBackground(): StateListDrawable {
		return StateListDrawable().apply {
			addState(intArrayOf(android.R.attr.state_pressed), createRoundedRect(
				fillColor = Color.argb(160, 86, 198, 243),
				strokeColor = Color.argb(192, 145, 224, 255),
				cornerRadius = dp(8).toFloat(),
			))
			addState(intArrayOf(), createRoundedRect(
				fillColor = Color.argb(64, 15, 23, 42),
				strokeColor = Color.argb(64, 148, 163, 184),
				cornerRadius = dp(8).toFloat(),
			))
		}
	}

	private fun createRoundedRect(fillColor: Int, strokeColor: Int, cornerRadius: Float): GradientDrawable {
		return GradientDrawable().apply {
			shape = GradientDrawable.RECTANGLE
			this.cornerRadius = cornerRadius
			setColor(fillColor)
			setStroke(dp(1), strokeColor)
		}
	}

	private fun controlTextColors(): ColorStateList {
		return ColorStateList(
			arrayOf(intArrayOf(android.R.attr.state_pressed), intArrayOf()),
			intArrayOf(
				Color.argb(255, 11, 17, 32),
				Color.argb(224, 255, 255, 255),
			),
		)
	}

	private fun View.withLayoutMargins(left: Int, top: Int, right: Int, bottom: Int): View {
		layoutParams = (layoutParams as ViewGroup.MarginLayoutParams).apply {
			setMargins(left, top, right, bottom)
		}
		return this
	}

	private fun enterImmersiveMode() {
		if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
			WindowCompat.setDecorFitsSystemWindows(window, false)
			window.insetsController?.hide(
				android.view.WindowInsets.Type.statusBars() or android.view.WindowInsets.Type.navigationBars(),
			)
		}
		else {
			@Suppress("DEPRECATION")
			window.decorView.systemUiVisibility = (
				View.SYSTEM_UI_FLAG_LAYOUT_STABLE
					or View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION
					or View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN
					or View.SYSTEM_UI_FLAG_HIDE_NAVIGATION
					or View.SYSTEM_UI_FLAG_FULLSCREEN
					or View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY
			)
		}
	}

	private fun resolveStoragePath(relativePath: String): File {
		val cleanPath = relativePath.removePrefix("/")
		require(!cleanPath.contains("..")) { "Invalid path: $relativePath" }
		return File(File(filesDir, "chipdx"), cleanPath)
	}

	private fun updateTouchControls(controls: FrameLayout, event: MotionEvent) {
		if (event.actionMasked == MotionEvent.ACTION_CANCEL) {
			clearTouchInputs()
			return
		}

		var nextMask = 0
		nextMask = nextMask or resolveDirectionPadBit(controls, event)

		val rect = Rect()
		for (touchButton in touchButtons) {
			rect.set(0, 0, touchButton.view.width, touchButton.view.height)
			controls.offsetDescendantRectToMyCoords(touchButton.view, rect)
			var pressed = false
			for (pointerIndex in 0 until event.pointerCount) {
				if (!isPointerActive(event, pointerIndex)) {
					continue
				}
				val x = event.getX(pointerIndex).toInt()
				val y = event.getY(pointerIndex).toInt()
				if (rect.contains(x, y)) {
					pressed = true
					nextMask = nextMask or touchButton.bit
					break
				}
			}
			touchButton.view.isPressed = pressed
		}
		touchButtonsMask = nextMask
	}

	private fun resolveDirectionPadBit(controls: FrameLayout, event: MotionEvent): Int {
		val pad = directionPad ?: return 0
		val rect = Rect(0, 0, pad.view.width, pad.view.height)
		controls.offsetDescendantRectToMyCoords(pad.view, rect)

		var activeBit = 0
		for (pointerIndex in 0 until event.pointerCount) {
			if (!isPointerActive(event, pointerIndex)) {
				continue
			}

			val x = event.getX(pointerIndex)
			val y = event.getY(pointerIndex)
			if (!rect.contains(x.toInt(), y.toInt())) {
				continue
			}

			activeBit = getPadDirectionBit(rect, x, y)
			if (activeBit != 0) {
				break
			}
		}

		pad.up.isPressed = activeBit == INPUT_UP
		pad.left.isPressed = activeBit == INPUT_LEFT
		pad.down.isPressed = activeBit == INPUT_DOWN
		pad.right.isPressed = activeBit == INPUT_RIGHT
		return activeBit
	}

	private fun getPadDirectionBit(rect: Rect, x: Float, y: Float): Int {
		val dx = x - rect.exactCenterX()
		val dy = y - rect.exactCenterY()
		val minDim = min(rect.width(), rect.height()).toFloat()
		val deadzone = minDim * 0.09f
		if ((dx * dx) + (dy * dy) < deadzone * deadzone) {
			return 0
		}
		return if (abs(dx) > abs(dy)) {
			if (dx > 0f) INPUT_RIGHT else INPUT_LEFT
		}
		else {
			if (dy > 0f) INPUT_DOWN else INPUT_UP
		}
	}

	private fun isPointerActive(event: MotionEvent, pointerIndex: Int): Boolean {
		return when (event.actionMasked) {
			MotionEvent.ACTION_UP,
			MotionEvent.ACTION_POINTER_UP -> pointerIndex != event.actionIndex
			else -> true
		}
	}

	private fun clearTouchInputs() {
		touchButtonsMask = 0
		directionPad?.up?.isPressed = false
		directionPad?.left?.isPressed = false
		directionPad?.down?.isPressed = false
		directionPad?.right?.isPressed = false
		for (touchButton in touchButtons) {
			touchButton.view.isPressed = false
		}
	}

	private fun clearAllInputs() {
		keyButtonsMask = 0
		clearTouchInputs()
	}

	private fun setKeyBit(bit: Int, enabled: Boolean) {
		keyButtonsMask = if (enabled) {
			keyButtonsMask or bit
		}
		else {
			keyButtonsMask and bit.inv()
		}
	}

	private fun dp(value: Int): Int {
		return (value * resources.displayMetrics.density).toInt()
	}

	private fun mapKey(keyCode: Int): Int? {
		return when (keyCode) {
			KeyEvent.KEYCODE_DPAD_UP,
			KeyEvent.KEYCODE_W -> INPUT_UP
			KeyEvent.KEYCODE_DPAD_LEFT,
			KeyEvent.KEYCODE_A -> INPUT_LEFT
			KeyEvent.KEYCODE_DPAD_DOWN,
			KeyEvent.KEYCODE_S -> INPUT_DOWN
			KeyEvent.KEYCODE_DPAD_RIGHT,
			KeyEvent.KEYCODE_D -> INPUT_RIGHT
			KeyEvent.KEYCODE_SPACE,
			KeyEvent.KEYCODE_BUTTON_A -> INPUT_A
			KeyEvent.KEYCODE_DEL,
			KeyEvent.KEYCODE_BUTTON_B -> INPUT_B
			KeyEvent.KEYCODE_ENTER,
			KeyEvent.KEYCODE_BUTTON_START -> INPUT_START
			KeyEvent.KEYCODE_SHIFT_LEFT,
			KeyEvent.KEYCODE_SHIFT_RIGHT,
			KeyEvent.KEYCODE_BUTTON_SELECT -> INPUT_SELECT
			else -> null
		}
	}

	private companion object {
		const val INPUT_UP = 1 shl 0
		const val INPUT_LEFT = 1 shl 1
		const val INPUT_DOWN = 1 shl 2
		const val INPUT_RIGHT = 1 shl 3
		const val INPUT_A = 1 shl 4
		const val INPUT_B = 1 shl 5
		const val INPUT_START = 1 shl 6
		const val INPUT_SELECT = 1 shl 7
	}
}
