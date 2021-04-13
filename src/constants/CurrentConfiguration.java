package constants;

import java.awt.Color;

import executionThreads.PopupWindow;

public class CurrentConfiguration {
	public static final boolean fadeIn = false;
	public static final boolean slideIn = false;
	public static final boolean darkMode = false;
	
	//PopupWindow
	public static PopupWindow popupWindow;
	
	//Button Messages
	public static String quitButtonMsg = "I'm Done stretching!";
	public static String terminateButtonMsg = "Stop the timer";
	
	//Label Messages
	public static String msgHeading = "Up you go!";
	public static String msgBody = "Time to stretch";
	
	//program running
	public static int buttonDisableDuration = 30*1000;	//in milliseconds
	public static int timerDuration = 30;	//in minutes
	
	
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
	
	//Set color scheme before
	public static void setDarkMode() {
		System.out.println("###Running setDarkMode()");
		if(darkMode == false) {
			//light mode color scheme
				//common scheme
				bg = Constants.lightModeBg;
				fg = Constants.lightModeFg;
			
				//button scheme
				buttonBg = Constants.lightModeButtonBg;
				buttonHoverBg = Constants.lightModeButtonHoverBg;
				buttonFg = Constants.lightModeButtonFg;
				
				//label scheme
				labelBg = Constants.lightModeLabelBg;
		}
		else
		{
			//dark mode color scheme
			//common scheme
			bg = Constants.darkModeBg;
			System.out.println(bg);
			fg = Constants.darkModeFg;
		
			//button scheme
			buttonBg = Constants.darkModeButtonBg;

			buttonHoverBg = Constants.darkModeButtonHoverBg;
			buttonFg = Constants.darkModeButtonFg;
			
			//label scheme
			labelBg = Constants.darkModeLabelBg;
		}
	}
}
