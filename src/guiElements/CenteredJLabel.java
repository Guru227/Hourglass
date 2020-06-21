package guiElements;

import java.awt.Color;
import java.awt.Font;
import java.awt.GridBagLayout;

import javax.swing.JLabel;
import javax.swing.JPanel;


import constants.Constants;
import constants.CurrentConfiguration;

public class CenteredJLabel extends JPanel{
    private GridBagLayout layout = new GridBagLayout();
    private JLabel label;

    public CenteredJLabel(String message, Font labelFont) {
    	label = new JLabel(message);
    	label.setFont(labelFont);
    	if(CurrentConfiguration.darkMode) {
    		label.setForeground(Constants.darkModeFg);
    		label.setBackground(Constants.darkModeLabelBg);
    	}
        setLayout(layout);
        add(label);  	
    }
}