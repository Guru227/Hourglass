package executionThreads;

import java.awt.GridBagConstraints;
import java.awt.GridBagLayout;
import java.awt.Insets;

import javax.swing.JFrame;
import javax.swing.JPanel;

import guiElements.CenteredJLabel;
import guiElements.ButtonLayout;

import constants.Constants;
import constants.CurrentConfiguration;


public class PopupWindow extends JFrame{

	/**
	 * 
	 */
	private static final long serialVersionUID = 2238614558467991706L;
	private Canvas canvas = new Canvas();
	
	//initialize window method
	private void initializeWindow()
	{
		//initialize window
		setSize(Constants.windowWidth, Constants.windowHeight);
		setLocationRelativeTo(null);	//centers window
		setUndecorated(true);	//removes title bar
		setAlwaysOnTop(true);	//overlays this window over all others
		Constants.popupWindow = this;
	}	

	//Class constructor
 	PopupWindow()
	{
		initializeWindow();
		add(canvas);
	}
 	public void openWindow() {
		setVisible(true);	//This has to be last	
 	}
 	public void closeWindow() {
		setVisible(false);	//This has to be last	
 	}

}


class Canvas extends JPanel{
	/**
	 * 
	 */
	private static final long serialVersionUID = -2011834783476562108L;

	private GridBagLayout messageBodyLayout = new GridBagLayout();
	
	//Message Heading Layout constraints
	public static GridBagConstraints msgHeadingConstraints = new GridBagConstraints();
	//Message Body Layout Constraints
	public static GridBagConstraints msgBodyConstraints = new GridBagConstraints();
	//Button Layout Constraints
	public static GridBagConstraints buttonLayoutConstraints = new GridBagConstraints();

	//Define Labels
	private CenteredJLabel headLabel = new CenteredJLabel(Constants.msgHeading, Constants.msgHeadingFont);
	private CenteredJLabel bodyLabel = new CenteredJLabel(Constants.msgBody, Constants.msgBodyFont);
	private ButtonLayout b = new ButtonLayout();
	
	private void initializeConstraints() {
		//Message Heading Layout constraints
		msgHeadingConstraints.gridx = 0;
		msgHeadingConstraints.gridy = 0;
		msgHeadingConstraints.gridwidth = 5;
		msgHeadingConstraints.insets = new Insets((int) Constants.windowHeight/11, 0, 0, 0);
		
		//Message Body Layout Constraints
		msgBodyConstraints.gridx = 0;
		msgBodyConstraints.gridy = 1;
		msgBodyConstraints.gridwidth = 5;
		msgBodyConstraints.insets = new Insets(0, 0, 0, 0);
		
		//Button Layout Constraints
		buttonLayoutConstraints.gridx = 0;
		buttonLayoutConstraints.gridy = 2;
		buttonLayoutConstraints.insets = new Insets(Constants.windowHeight/5,0,0,0);
	}
	
	Canvas(){
		//Initialize MessageBody Panel
		setLayout(messageBodyLayout);
		initializeConstraints();
		
		if(CurrentConfiguration.darkMode) {
			setBackground(Constants.darkModeBg);	
			headLabel.setBackground(Constants.darkModeBg);
			bodyLabel.setBackground(Constants.darkModeBg);
		}
		else {
			setBackground(Constants.lightModeBg);	
			headLabel.setBackground(Constants.lightModeBg);
			bodyLabel.setBackground(Constants.lightModeBg);
		}
		add(headLabel, msgHeadingConstraints);
		add(bodyLabel, msgBodyConstraints);
		add(b, buttonLayoutConstraints);
	}
}


