package constants;

import buttons.CloseButton;
import executionThreads.PopupWindow;

public final class DefaultHourglassConfiguration {
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
	public static boolean programRunning = true;
	public static CloseButton disabledButton;
	public static boolean continueTimer = false;
	public static int buttonDisableDuration = 30*1000;	//in milliseconds
	public static int timerDuration = 30;	//in minutes
}
