package constants;

import java.awt.Color;
import java.awt.Dimension;
import java.awt.Font;
import java.awt.GridBagConstraints;
import java.awt.Insets;
import java.awt.Toolkit;

import buttons.CloseButton;
import executionThreads.PopupWindow;

public class Constants {
	//Screen and Window Dimensions
	public static Dimension screenSize = Toolkit.getDefaultToolkit().getScreenSize();
	public static int screenWidth = (int) screenSize.getWidth();
	public static int screenHeight= (int) screenSize.getHeight();
	public static int windowWidth = screenWidth * 2/3;
	public static int windowHeight= screenHeight* 2/5;
	
	//PopupWindow
	public static PopupWindow popupWindow;
	
	//msgHeading Font
	public static int msgHeadingFontSize = windowHeight/5;
	public static String msgHeadingFontName = "Verdana";
	public static Font msgHeadingFont = new Font(msgHeadingFontName, Font.PLAIN, msgHeadingFontSize);
	
	//msgBody Font
	public static int msgBodyFontSize = windowHeight/11;
	public static String msgBodyFontName = "Verdana";
	public static Font msgBodyFont = new Font(msgBodyFontName, Font.PLAIN, msgBodyFontSize);
	
	//Button Font
	public static int buttonFontSize = windowHeight/22;
	public static String buttonFontName = "Arial";
	public static Font buttonFont = new Font(buttonFontName, Font.PLAIN, buttonFontSize);
	
	//Button Messages
	public static String quitButtonMsg = "I'm Done stretching!";
	public static String terminateButtonMsg = "Stop the timer";
	
	//Label Messages
	public static String msgHeading = "Up you go!";
	public static String msgBody = "Time to stretch";

	//light mode color scheme
	//public static lightModeBgColor = popupWindow.getBackground();
	
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
	public static boolean programRunning = true;
	public static CloseButton disabledButton;
	public static boolean continueTimer = false;
	public static int buttonDisableDuration = 30*1000;	//in milliseconds
	public static int timerDuration = 30;	//in minutes
	
}
