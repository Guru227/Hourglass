package executionThreads;

import java.awt.GridBagConstraints;
import java.awt.GridBagLayout;
import java.awt.Insets;
import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.TimeUnit;

import javax.swing.JDialog;
import javax.swing.JPanel;

import guiElements.CenteredJLabel;
import guiElements.ButtonLayout;

import constants.CurrentConfiguration;

public class PopupWindow extends JDialog{
	private static final long serialVersionUID = 2238614558467991706L;
	private final static ScheduledExecutorService scheduler = Executors.newScheduledThreadPool(1);
	private Canvas canvas = new Canvas();	
	
	//initialize window method
	private void initializeWindow()
	{
		//initialize window
		setSize(CurrentConfiguration.windowWidth, CurrentConfiguration.windowHeight);
		setLocationRelativeTo(null);	//centers window
		setModalityType(JDialog.DEFAULT_MODALITY_TYPE);	//setModal(true); pauses thread execution
		setUndecorated(true);	//removes title bar
		setAlwaysOnTop(true);	//overlays this window over all others
		CurrentConfiguration.popupWindow = this;
	}
	
	//Class constructor
 	PopupWindow()
	{
		initializeWindow();
		add(canvas);
		CurrentConfiguration.parentJDialog = this;	
	}
 	
 	public void openWindow() {
 		System.out.println("\tStarting window");
 		System.out.println("\tAlready starting timer");
 		
 		MyTimerTask t = new MyTimerTask(CurrentConfiguration.b1);
 		CurrentConfiguration.ta = t;
 		System.out.println("\t\t" + CurrentConfiguration.buttonDisableDuration);
 		scheduler.schedule(CurrentConfiguration.ta, CurrentConfiguration.timerDuration, TimeUnit.SECONDS);
 		
		setVisible(true);	//This has to be last	
 	}
}

//	GUI Layout
class Canvas extends JPanel {
	private static final long serialVersionUID = -2011834783476562108L;

	private GridBagLayout messageBodyLayout = new GridBagLayout();
	
	//Message Heading Layout constraints
	public static GridBagConstraints msgHeadingConstraints = new GridBagConstraints();
	//Message Body Layout Constraints
	public static GridBagConstraints msgBodyConstraints = new GridBagConstraints();
	//Button Layout Constraints
	public static GridBagConstraints buttonLayoutConstraints = new GridBagConstraints();

	//Define Labels
	private CenteredJLabel headLabel = new CenteredJLabel(CurrentConfiguration.msgHeading, CurrentConfiguration.msgHeadingFont);
	private CenteredJLabel bodyLabel = new CenteredJLabel(CurrentConfiguration.msgBody, CurrentConfiguration.msgBodyFont);
	private ButtonLayout b = new ButtonLayout();
	
	private void initializeConstraints() {
		//Message Heading Layout constraints
		msgHeadingConstraints.gridx = 0;
		msgHeadingConstraints.gridy = 0;
		msgHeadingConstraints.gridwidth = 5;
		msgHeadingConstraints.insets = new Insets((int) CurrentConfiguration.windowHeight/11, 0, 0, 0);
		
		//Message Body Layout Constraints
		msgBodyConstraints.gridx = 0;
		msgBodyConstraints.gridy = 1;
		msgBodyConstraints.gridwidth = 5;
		msgBodyConstraints.insets = new Insets(0, 0, 0, 0);
		
		//Button Layout Constraints
		buttonLayoutConstraints.gridx = 0;
		buttonLayoutConstraints.gridy = 2;
		buttonLayoutConstraints.insets = new Insets(CurrentConfiguration.windowHeight/5,0,0,0);
	}
	
	Canvas(){
		//Initialize MessageBody Panel
		setLayout(messageBodyLayout);
		initializeConstraints();
		
		//Set background colors
		setBackground(CurrentConfiguration.bg);	
		headLabel.setBackground(CurrentConfiguration.bg);
		bodyLabel.setBackground(CurrentConfiguration.bg);
		
		add(headLabel, msgHeadingConstraints);
		add(bodyLabel, msgBodyConstraints);
		add(b, buttonLayoutConstraints);
	}
}


