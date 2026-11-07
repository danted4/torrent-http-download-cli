package com.danted4.torrenter

import android.Manifest
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.os.Environment
import android.os.PowerManager
import android.provider.Settings
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.delay
import uniffi.torrenter.Engine
import uniffi.torrenter.JobView
import uniffi.torrenter.Kind
import uniffi.torrenter.banner

class MainActivity : ComponentActivity() {
    private val notify =
        registerForActivityResult(ActivityResultContracts.RequestPermission()) { }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        if (Build.VERSION.SDK_INT >= 33) {
            notify.launch(Manifest.permission.POST_NOTIFICATIONS)
        }
        val prefs = getSharedPreferences(Prefs.NAME, Context.MODE_PRIVATE)
        setContent {
            var themeId by remember { mutableStateOf(Prefs.theme(prefs)) }
            val colors = palette(themeId)
            MaterialTheme(
                colorScheme =
                    darkColorScheme(
                        primary = colors.fg,
                        onPrimary = colors.bg,
                        background = colors.bg,
                        surface = colors.row,
                        onBackground = colors.fg,
                        onSurface = colors.fg,
                        error = colors.fail,
                    ),
            ) {
                Surface(modifier = Modifier.fillMaxSize(), color = colors.bg) {
                    AppScreen(
                        activity = this,
                        palette = colors,
                        themeId = themeId,
                        onTheme = { id ->
                            themeId = id
                            Prefs.saveTheme(prefs, id)
                        },
                    )
                }
            }
        }
    }
}

@androidx.compose.runtime.Composable
private fun AppScreen(
    activity: MainActivity,
    palette: Palette,
    themeId: ThemeId,
    onTheme: (ThemeId) -> Unit,
) {
    val app = activity.application as TorrenterApp
    val prefs = activity.getSharedPreferences(Prefs.NAME, Context.MODE_PRIVATE)
    var allowed by remember { mutableStateOf(Environment.isExternalStorageManager()) }
    var notice by remember { mutableStateOf<String?>(null) }
    var batteryWarned by remember { mutableStateOf(false) }
    var input by remember { mutableStateOf("") }
    var magnet by remember { mutableStateOf(true) }
    var seed by remember { mutableStateOf(Prefs.seed(prefs)) }
    var jobs by remember { mutableStateOf(listOf<JobView>()) }
    var engine by remember { mutableStateOf<Engine?>(null) }
    val buttons =
        ButtonDefaults.buttonColors(
            containerColor = palette.rowAlt,
            contentColor = palette.fg,
            disabledContainerColor = palette.row,
            disabledContentColor = palette.dim,
        )

    fun refresh() {
        val live = engine ?: return
        app.takeNotices().lastOrNull()?.let { notice = it }
        jobs = live.jobs()
        if (live.anyActive()) {
            DownloadService.startFromUi(activity)
        }
    }

    fun setShare(value: Boolean) {
        seed = value
        engine?.setSeed(value)
        Prefs.saveSeed(prefs, value)
    }

    LaunchedEffect(allowed) {
        if (!allowed) {
            return@LaunchedEffect
        }
        try {
            val opened = app.openEngine(seed)
            engine = opened
            seed = opened.seed()
            if (!batteryWarned && !isIgnoringBattery(activity)) {
                notice =
                    "the OS may stop seeding in the background unless battery optimization is ignored"
                requestBatteryExemption(activity)
                batteryWarned = true
            }
            refresh()
        } catch (error: Exception) {
            notice = error.message
        }
    }

    LaunchedEffect(engine) {
        if (engine == null) {
            return@LaunchedEffect
        }
        while (true) {
            delay(200)
            refresh()
        }
    }

    Column(
        modifier = Modifier.fillMaxSize().padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            Image(
                painter = painterResource(R.drawable.ic_brand),
                contentDescription = "torrenter",
                modifier = Modifier.size(28.dp),
            )
            Text(
                banner(),
                color = palette.fg,
                fontFamily = FontFamily.Monospace,
                style = MaterialTheme.typography.titleMedium,
                modifier = Modifier.weight(1f),
            )
            ThemeSwatches(selected = themeId, onSelect = onTheme)
        }
        if (!allowed) {
            Text("All files access is required so downloads go to Download/torrenter.", color = palette.dim)
            Button(onClick = { requestAllFiles(activity) }, colors = buttons, shape = RoundedCornerShape(2.dp)) {
                Text("grant all files access", fontFamily = FontFamily.Monospace)
            }
            Button(
                onClick = { allowed = Environment.isExternalStorageManager() },
                colors = buttons,
                shape = RoundedCornerShape(2.dp),
            ) { Text("I granted it", fontFamily = FontFamily.Monospace) }
            return@Column
        }
        notice?.let { Text(it, color = palette.fail, fontFamily = FontFamily.Monospace, fontSize = 13.sp) }
        ModeSwitch(magnet = magnet, palette = palette, onToggle = { magnet = !magnet })
        OutlinedTextField(
            value = input,
            onValueChange = { input = it },
            modifier = Modifier.fillMaxWidth(),
            placeholder = {
                Text(
                    if (magnet) "magnet:?xt=urn:btih:…" else "https://example.com/file.iso",
                    fontFamily = FontFamily.Monospace,
                )
            },
            singleLine = true,
            textStyle =
                androidx.compose.ui.text.TextStyle(
                    color = palette.fg,
                    fontFamily = FontFamily.Monospace,
                    fontSize = 14.sp,
                ),
            colors =
                OutlinedTextFieldDefaults.colors(
                    focusedBorderColor = palette.fg,
                    unfocusedBorderColor = palette.dim,
                    focusedTextColor = palette.fg,
                    unfocusedTextColor = palette.fg,
                    cursorColor = palette.fg,
                    focusedPlaceholderColor = palette.dim,
                    unfocusedPlaceholderColor = palette.dim,
                    focusedContainerColor = palette.row,
                    unfocusedContainerColor = palette.row,
                ),
            shape = RoundedCornerShape(2.dp),
        )
        Row(
            modifier = Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            SeedToggle(seed = seed, enabled = magnet, palette = palette, onChange = { setShare(it) })
            Button(
                onClick = {
                    val live = engine ?: return@Button
                    try {
                        live.queue(if (magnet) Kind.MAGNET else Kind.HTTP, input.trim())
                        input = ""
                        notice = null
                        refresh()
                    } catch (error: Exception) {
                        notice = error.message
                    }
                },
                colors = buttons,
                shape = RoundedCornerShape(2.dp),
                modifier = Modifier.weight(1f),
            ) { Text("START", fontFamily = FontFamily.Monospace) }
        }
        Text("Download/torrenter", color = palette.dim, fontFamily = FontFamily.Monospace, fontSize = 12.sp)
        if (jobs.isEmpty()) {
            Text("no jobs yet — paste a magnet or HTTP URL", color = palette.dim, fontFamily = FontFamily.Monospace)
        } else {
            LazyColumn(verticalArrangement = Arrangement.spacedBy(10.dp), modifier = Modifier.weight(1f)) {
                items(jobs, key = { it.id.toString() }) { job ->
                    JobCard(
                        job = job,
                        palette = palette,
                        buttons = buttons,
                        onPause = {
                            engine?.togglePause(job.id)
                            refresh()
                        },
                        onStop = {
                            engine?.stop(job.id)
                            refresh()
                        },
                        onClear = {
                            engine?.remove(job.id)
                            refresh()
                        },
                    )
                }
            }
        }
    }
}

@androidx.compose.runtime.Composable
private fun JobCard(
    job: JobView,
    palette: Palette,
    buttons: androidx.compose.material3.ButtonColors,
    onPause: () -> Unit,
    onStop: () -> Unit,
    onClear: () -> Unit,
) {
    val fraction = jobBarFraction(job)
    val accent = if (job.active) mix(palette.fg, palette.dim, 0.65f) else palette.fg
    val lineColor = if (job.error != null) palette.fail else palette.dim
    Column(
        modifier =
            Modifier
                .fillMaxWidth()
                .background(palette.row, RoundedCornerShape(4.dp))
                .padding(10.dp),
        verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            SeedGlyph(
                share = job.seed,
                selected = true,
                faded = false,
                palette = palette,
            )
            Text(
                job.title.ifEmpty { job.source },
                color = accent,
                fontFamily = FontFamily.Monospace,
                fontSize = 13.sp,
                modifier = Modifier.weight(1f),
            )
        }
        if (fraction == null) {
            LinearProgressIndicator(
                modifier = Modifier.fillMaxWidth(),
                color = accent,
                trackColor = palette.rowAlt,
            )
        } else {
            LinearProgressIndicator(
                progress = { fraction },
                modifier = Modifier.fillMaxWidth(),
                color = accent,
                trackColor = palette.rowAlt,
            )
        }
        Text(jobStatsLine(job), color = lineColor, fontFamily = FontFamily.Monospace, fontSize = 12.sp)
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            if (job.active) {
                Button(onClick = onPause, colors = buttons, shape = RoundedCornerShape(2.dp)) {
                    Text(if (job.paused) "resume" else "pause", fontFamily = FontFamily.Monospace)
                }
                Button(onClick = onStop, colors = buttons, shape = RoundedCornerShape(2.dp)) {
                    Text("stop", fontFamily = FontFamily.Monospace)
                }
            } else {
                Button(onClick = onClear, colors = buttons, shape = RoundedCornerShape(2.dp)) {
                    Text("clear", fontFamily = FontFamily.Monospace)
                }
            }
        }
    }
}

private fun jobBarFraction(job: JobView): Float? {
    val status = job.status
    if (status == "queued" || status == "starting" || status.contains("Initializing")) {
        return null
    }
    if (job.total <= 1uL && job.downloaded > job.total) {
        return null
    }
    if (job.total == 0uL) {
        return 0f
    }
    return (job.downloaded.toDouble() / job.total.toDouble()).toFloat().coerceIn(0f, 1f)
}

private fun mix(
    a: androidx.compose.ui.graphics.Color,
    b: androidx.compose.ui.graphics.Color,
    towardB: Float,
): androidx.compose.ui.graphics.Color {
    val t = towardB.coerceIn(0f, 1f)
    return androidx.compose.ui.graphics.Color(
        red = a.red * (1f - t) + b.red * t,
        green = a.green * (1f - t) + b.green * t,
        blue = a.blue * (1f - t) + b.blue * t,
        alpha = 1f,
    )
}

private fun requestAllFiles(activity: ComponentActivity) {
    val intent =
        Intent(Settings.ACTION_MANAGE_APP_ALL_FILES_ACCESS_PERMISSION).apply {
            data = Uri.parse("package:${activity.packageName}")
        }
    activity.startActivity(intent)
}

private fun isIgnoringBattery(activity: ComponentActivity): Boolean {
    val pm = activity.getSystemService(PowerManager::class.java)
    return pm.isIgnoringBatteryOptimizations(activity.packageName)
}

private fun requestBatteryExemption(activity: ComponentActivity) {
    val intent =
        Intent(Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS).apply {
            data = Uri.parse("package:${activity.packageName}")
        }
    runCatching { activity.startActivity(intent) }
}

