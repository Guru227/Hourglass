package constants;

import java.awt.Color;
import java.awt.Dimension;
import java.awt.Font;
import java.awt.Toolkit;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileNotFoundException;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.util.Properties;

import javax.swing.JDialog;

import executionThreads.MyTimerTask;
import executionThreads.PopupWindow;
import guiElements.CloseButton;

public class CurrentConfiguration {
	// screen and window dimensions
	public static Dimension screenSize = Toolkit.getDefaultToolkit().getScreenSize();
	public static int screenWidth = (int) screenSize.getWidth();
	public static int screenHeight= (int) screenSize.getHeight();
	public static int windowWidth = screenWidth * 2/3;
	public static int windowHeight= screenHeight* 2/5;
	
	// msgHeading Font
	public static int msgHeadingFontSize = windowHeight/5;
	public static String msgHeadingFontName = "Verdana";
	public static Font msgHeadingFont = new Font(msgHeadingFontName, Font.PLAIN, msgHeadingFontSize);
	
	// msgBody Font
	public static int msgBodyFontSize = windowHeight/11;
	public static String msgBodyFontName = "Verdana";
	public static Font msgBodyFont = new Font(msgBodyFontName, Font.PLAIN, msgBodyFontSize);
	
	// button Font
	public static int buttonFontSize = windowHeight/22;
	public static String buttonFontName = "Arial";
	public static Font buttonFont = new Font(buttonFontName, Font.PLAIN, buttonFontSize);

	
	public static boolean fadeIn;
	public static boolean slideIn;
	public static boolean darkMode;
	
	//Button Messages
	public static String quitButtonMsg;
	public static String terminateButtonMsg;
	
	//Label Messages
	public static String msgHeading;
	public static String msgBody;
	
	//color scheme decided depending on color mode	
	//common scheme
	public static volatile Color bg;
	public static volatile Color fg;

	//button scheme
	public static volatile Color buttonBg;
	public static volatile Color buttonHoverBg;
	public static volatile Color buttonFg;
	
	//label scheme
	public static volatile Color labelBg;
	
	//program running
	public static PopupWindow popupWindow;
	public static boolean programRunning = true;
	public static CloseButton disabledButton;
	public static JDialog parentJDialog;
	public static MyTimerTask ta;
	public static CloseButton b1;
	public static boolean continueTimer = false;
	public static int buttonDisableDuration;	//in milliseconds
	public static int timerDuration;	//in seconds
	
	//Set color scheme before
	public static void setDarkMode() {
		System.out.println("###Running setDarkMode()");
		if(darkMode) {
			//dark mode color scheme
			//common scheme
			bg = DefaultHourglassConfiguration.darkModeBg;
			//System.out.println(bg);
			fg = DefaultHourglassConfiguration.darkModeFg;
		
			//button scheme
			buttonBg = DefaultHourglassConfiguration.darkModeButtonBg;

			buttonHoverBg = DefaultHourglassConfiguration.darkModeButtonHoverBg;
			buttonFg = DefaultHourglassConfiguration.darkModeButtonFg;
			
			//label scheme
			labelBg = DefaultHourglassConfiguration.darkModeLabelBg;
		}
		else {
			//light mode color scheme
			//common scheme
			bg = DefaultHourglassConfiguration.lightModeBg;
			fg = DefaultHourglassConfiguration.lightModeFg;
		
			//button scheme
			buttonBg = DefaultHourglassConfiguration.lightModeButtonBg;
			buttonHoverBg = DefaultHourglassConfiguration.lightModeButtonHoverBg;
			buttonFg = DefaultHourglassConfiguration.lightModeButtonFg;
			
			//label scheme
			labelBg = DefaultHourglassConfiguration.lightModeLabelBg;
		}
	}
		
	public static void loadDefaultConfiguration() {
		File configFile = new File("defualtConfig.xml");
		Properties props = new Properties();
		
		try {
			InputStream inputStream = new FileInputStream(configFile);
			props.loadFromXML(inputStream);
			
			System.out.println("Successfully loaded XML config file");
			// button messages
			quitButtonMsg = props.getProperty(ConfigurationHeaders.QuitButtonMsg);
			terminateButtonMsg = props.getProperty(ConfigurationHeaders.TerminateButtonMsg);
			
			// label messages
			msgHeading = props.getProperty(ConfigurationHeaders.MsgHeading);
			msgBody = props.getProperty(ConfigurationHeaders.MsgBody);

			//options
			fadeIn = Boolean.valueOf(props.getProperty(ConfigurationHeaders.FadeIn));
			slideIn = Boolean.valueOf(props.getProperty(ConfigurationHeaders.SlideIn));
			darkMode = Boolean.valueOf(props.getProperty(ConfigurationHeaders.DarkMode));
			
			//Program Running
			buttonDisableDuration = (int) Integer.parseInt(props.getProperty(ConfigurationHeaders.ButtonDisableDuration));
			timerDuration = (int) Integer.parseInt(props.getProperty(ConfigurationHeaders.TimerDuration));
			
			System.out.println("\t\t" + buttonDisableDuration + "\t" + timerDuration);
			
			OutputStream outputStream = new FileOutputStream(configFile);
			props.storeToXML(outputStream, "Default settings");
			inputStream.close();
			
		} catch (FileNotFoundException fnfe) {
			DefaultHourglassConfiguration.saveDefaultConfiguration();
			System.out.println("FileNotFoundException\nAttempting to restart\n");
			loadDefaultConfiguration();
		} catch (IOException ex) {
			
		}
		//set dark mode based on configuration
		setDarkMode();
	}
	
}
