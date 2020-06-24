package buttons;

import javax.swing.JButton;

import constants.Constants;
import constants.CurrentConfiguration;

public class MenuButton extends JButton
{
	/**
	 * 
	 */
	private static final long serialVersionUID = -7026716914176317123L;

	MenuButton(){
		setBounds(0,0,Constants.buttonFontSize,Constants.buttonFontSize);	
		
		//customize look and feel
		setBorderPainted(false);	//removes borders
		setFocusPainted(false);		//removes default color
		setFont(Constants.buttonFont);
		
		if(CurrentConfiguration.darkMode) {
			setBackground(Constants.darkModeButtonBg);
	        setForeground(Constants.darkModeButtonFg);
	        //Hover effects
	        addMouseListener(new java.awt.event.MouseAdapter() {
		        public void mouseEntered(java.awt.event.MouseEvent evt) {
		        	setBackground(Constants.darkModeButtonHoverBg);
		        }

		        public void mouseExited(java.awt.event.MouseEvent evt) {
		            setBackground(Constants.darkModeButtonBg);
		        }
		    });
		}
		else {
			setBackground(Constants.lightModeButtonBg);
	        setForeground(Constants.lightModeButtonFg);
	        addMouseListener(new java.awt.event.MouseAdapter() {
		        public void mouseEntered(java.awt.event.MouseEvent evt) {
		        	setBackground(Constants.lightModeButtonHoverBg);
		        }
	
		        public void mouseExited(java.awt.event.MouseEvent evt) {
		            setBackground(Constants.lightModeButtonBg);
		        }
		    });
		}   
	}
}
