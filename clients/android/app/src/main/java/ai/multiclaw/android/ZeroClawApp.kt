package ai.multiclaw.android

import android.app.Application
import android.app.NotificationChannel
import android.app.NotificationManager
import android.os.Build

class MultiClawApp : Application() {
    
    companion object {
        const val CHANNEL_ID = "multiclaw_service"
        const val CHANNEL_NAME = "MultiClaw Agent"
        const val AGENT_CHANNEL_ID = "multiclaw_agent"
        const val AGENT_CHANNEL_NAME = "Agent Messages"
    }
    
    override fun onCreate() {
        super.onCreate()
        createNotificationChannels()
        
        // TODO: Initialize native library
        // System.loadLibrary("multiclaw")
    }
    
    private fun createNotificationChannels() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val manager = getSystemService(NotificationManager::class.java)
            
            // Service channel (foreground service)
            val serviceChannel = NotificationChannel(
                CHANNEL_ID,
                CHANNEL_NAME,
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "MultiClaw background service"
                setShowBadge(false)
            }
            
            // Agent messages channel
            val agentChannel = NotificationChannel(
                AGENT_CHANNEL_ID,
                AGENT_CHANNEL_NAME,
                NotificationManager.IMPORTANCE_HIGH
            ).apply {
                description = "Messages from your AI agent"
                enableVibration(true)
            }
            
            manager.createNotificationChannel(serviceChannel)
            manager.createNotificationChannel(agentChannel)
        }
    }
}
