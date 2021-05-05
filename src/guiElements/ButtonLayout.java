package guiElements;

import java.awt.GridLayout;

import javax.swing.JPanel;

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
		
		b1 = new CloseButton(CurrentConfiguration.quitButtonMsg);
		CurrentConfiguration.b1 = b1;
		System.out.println("\tDisabling Button");
		b1.setEnabled(false);		
		
		b2 = new TerminateButton(CurrentConfiguration.terminateButtonMsg);
		p = new JPanel();
		p.setBackground(CurrentConfiguration.bg);
		
		add(b1);
		add(p);
		add(b2);		
		
	}
}

