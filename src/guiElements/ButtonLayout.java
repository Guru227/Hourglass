package guiElements;

import java.awt.Color;
import java.awt.GridLayout;
import java.util.concurrent.CountDownLatch;

import javax.swing.JPanel;

import buttons.CloseButton;
import executionThreads.PopupWindow;
import buttons.TerminateButton;
import constants.Constants;
import constants.CurrentConfiguration;

public class ButtonLayout extends JPanel{
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
		add(b1);
		add(p);
		add(b2);
	}
}
