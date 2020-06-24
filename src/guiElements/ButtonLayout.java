package guiElements;

import java.awt.GridLayout;

import javax.swing.JPanel;

import buttons.CloseButton;
import buttons.TerminateButton;
import constants.Constants;
import constants.CurrentConfiguration;

public class ButtonLayout extends JPanel{
	/**
	 * 
	 */
	private static final long serialVersionUID = -8151292879831467279L;
	private GridLayout ButtonLayout = new GridLayout(1, 3);
	private CloseButton b1;
	private TerminateButton b2;
	private JPanel p;
	
	public ButtonLayout(){	
		setLayout(ButtonLayout);
		
		b1 = new CloseButton(Constants.quitButtonMsg);
		b1.setEnabled(false);
		Constants.disabledButton = b1;
		b2 = new TerminateButton(Constants.terminateButtonMsg);
		p = new JPanel();
		if(CurrentConfiguration.darkMode) {
			p.setBackground(Constants.darkModeBg);
		}
		else {
			p.setBackground(Constants.lightModeBg);
		}
		add(b1);
		add(p);
		add(b2);
	}
}
