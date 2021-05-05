package constants;

public class ConfigurationHeaders {
	// config file name
	public static String ConfigFilename;
	
	//Screen and Window Dimensions
	public static String ScreenSize = "screenSize";
	public static String ScreenWidth = "screenWidth";
	public static String ScreenHeight= "screenHeight";
	public static String WindowWidth = "windowWidth";
	public static String WindowHeight = "windowHeight";
	
	//msgHeading Font
	public static String MsgHeadingFontSize = "msgHeadingFontSize";
	public static String MsgHeadingFontName = "msgHeadingFontName";
	public static String MsgHeadingFont = "msgHeadingFont";
	
	//msgBody Font
	public static String MsgBodyFontSize = "msgBodyFontSize";
	public static String MsgBodyFontName = "msgBodyFontName";
	public static String MsgBodyFont = "msgBodyFont";
	
	//Button Font
	public static String ButtonFontSize = "buttonFontSize";
	public static String ButtonFontName = "buttonFontName";
	public static String ButtonFont = "buttonFont";
	
	
	// button mMessages
	public static String QuitButtonMsg = "quitButtonMsg";
	public static String TerminateButtonMsg = "terminateButtonMsg";
	
	// label messages
	public static String MsgHeading = "msgHeading";
	public static String MsgBody = "msgBody";
	
	
	public static String FadeIn = "fadeIn";
	public static String SlideIn = "slideIn";
	public static String DarkMode = "darkMode";

	// color scheme decided depending on color mode	
	// common scheme
	public static String Bg = "bg";
	public static String Fg = "fg";

	// button scheme
	public static String ButtonBg = "buttonBg";
	public static String ButtonHoverBg = "buttonHoverBg";
	public static String ButtonFg = "buttonFg";
	
	// label scheme
	public static String LabelBg = "labelBg";	
	
	public static String ButtonDisableDuration = "buttonDisableDuration";	//in milliseconds
	public static String TimerDuration = "timerDuration";	//in seconds
	
	public static void setConfigFileName() {
		
	}
	
	public static void getConfigFileName() {
		
	}
}
