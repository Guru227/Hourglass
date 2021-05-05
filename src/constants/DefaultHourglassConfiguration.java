package constants;

import java.awt.Color;
import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.OutputStream;
import java.util.Properties;

public final class DefaultHourglassConfiguration {

	//Button Messages
	public static String quitButtonMsg = "I'm Done stretching!";
	public static String terminateButtonMsg = "Stop the timer";
	
	//Label Messages
	public static String msgHeading = "Up you go!";
	public static String msgBody = "Time to stretch";
	
	public static final boolean fadeIn = false;
	public static final boolean slideIn = false;
	public static final boolean darkMode = false;
	
	//light mode color scheme
		//common scheme
		public static Color lightModeBg = Color.decode("0xEEEEEE");	//Charcoal Grey
		public static Color lightModeFg = Color.BLACK;
	
		//button scheme
		public static Color lightModeButtonBg = Color.decode("0xFFFFFF");
		public static Color lightModeButtonHoverBg = Color.decode("0xDDDDDD");
		public static Color lightModeButtonFg = Color.BLACK;
		
		//label scheme
		public static Color lightModeLabelBg = Color.decode("0xEEEEEE");
	
	//dark mode color scheme
		//common scheme
		public static Color darkModeBg = Color.decode("0x444444");	//Charcoal Grey
		public static Color darkModeFg = Color.WHITE;
	
		//button scheme
		public static Color darkModeButtonBg = Color.decode("0x555555");
		public static Color darkModeButtonHoverBg = Color.decode("0x333333");
		public static Color darkModeButtonFg = Color.WHITE;
		
		//label scheme
		public static Color darkModeLabelBg = Color.decode("0x333333");
		
	//program running
	public static int buttonDisableDuration = 900;	//in seconds (15 minutes)
	public static int timerDuration = 30;	//in seconds
	
	public static void saveDefaultConfiguration() {
		File configFile = new File("defualtConfig.xml");
		
		try {
			Properties props = new Properties();					
			
			// button messages
			props.setProperty(ConfigurationHeaders.QuitButtonMsg, quitButtonMsg);
			props.setProperty(ConfigurationHeaders.TerminateButtonMsg, terminateButtonMsg);
			
			// label messages
			props.setProperty(ConfigurationHeaders.MsgHeading, msgHeading);
			props.setProperty(ConfigurationHeaders.MsgBody, msgBody);

			//options
			props.setProperty(ConfigurationHeaders.FadeIn, String.valueOf(fadeIn));
			props.setProperty(ConfigurationHeaders.SlideIn, String.valueOf(slideIn));
			props.setProperty(ConfigurationHeaders.DarkMode, String.valueOf(darkMode));
			
			//Program Running
			props.setProperty(ConfigurationHeaders.ButtonDisableDuration, String.valueOf(buttonDisableDuration));
			props.setProperty(ConfigurationHeaders.TimerDuration, String.valueOf(timerDuration));
			
			
			OutputStream outputStream = new FileOutputStream(configFile);
			props.storeToXML(outputStream, "Default settings");
			outputStream.close();
			
		} catch (IOException ex) {
			// I/O error
		}
	}
}
